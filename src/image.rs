use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::{
    class_table::ClassInfo,
    heap::{Generation, Heap, ObjectSnapshot},
    interpreter::Vm,
    object::{Format, HeaderWord, MethodHeaderFields, OVERFLOW_SIZE_SENTINEL},
    value::Oop,
};

const MAGIC: &[u8; 8] = b"SMTLIMG\0";
const VERSION: u32 = 3;
const NONE_U64: u64 = u64::MAX;

#[derive(Debug)]
pub enum ImageError {
    Io(std::io::Error),
    InvalidFormat(&'static str),
    InvalidVersion(u32),
    UnknownObjectOffset(u64),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::InvalidFormat(msg) => write!(f, "invalid image format: {msg}"),
            Self::InvalidVersion(version) => write!(f, "unsupported image version: {version}"),
            Self::UnknownObjectOffset(offset) => {
                write!(f, "unknown serialized object offset: {offset}")
            }
        }
    }
}

impl std::error::Error for ImageError {}

impl From<std::io::Error> for ImageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone)]
struct SerializedObject {
    offset: u64,
    snapshot: ObjectSnapshot,
}

#[derive(Clone)]
struct LoadedObject {
    header_raw: u64,
    slot_count: usize,
    generation: Generation,
    byte_len: Option<usize>,
    payload_words: Vec<u64>,
}

fn write_u8(writer: &mut impl Write, value: u8) -> Result<(), ImageError> {
    writer.write_all(&[value])?;
    Ok(())
}

fn write_u32(writer: &mut impl Write, value: u32) -> Result<(), ImageError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_u64(writer: &mut impl Write, value: u64) -> Result<(), ImageError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_u8(reader: &mut impl Read) -> Result<u8, ImageError> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32(reader: &mut impl Read) -> Result<u32, ImageError> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(reader: &mut impl Read) -> Result<u64, ImageError> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn write_string(writer: &mut impl Write, value: &str) -> Result<(), ImageError> {
    write_u64(writer, value.len() as u64)?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

fn read_string(reader: &mut impl Read) -> Result<String, ImageError> {
    let len = read_u64(reader)? as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn object_size_bytes(snapshot: &ObjectSnapshot) -> u64 {
    let overflow_words = u64::from(snapshot.slot_count >= OVERFLOW_SIZE_SENTINEL);
    (1 + snapshot.slot_count as u64 + overflow_words) * 8
}

fn encode_oop(oop: Oop, offset_by_raw: &HashMap<u64, u64>) -> Result<u64, ImageError> {
    if oop.is_heap_ptr() {
        offset_by_raw
            .get(&oop.raw())
            .copied()
            .ok_or(ImageError::UnknownObjectOffset(oop.raw()))
    } else {
        Ok(oop.raw())
    }
}

fn decode_oop(raw: u64, offset_to_oop: &HashMap<u64, Oop>) -> Result<Oop, ImageError> {
    if raw == 0 || (raw & 1) == 1 {
        Ok(Oop::from_raw(raw))
    } else {
        offset_to_oop
            .get(&raw)
            .copied()
            .ok_or(ImageError::UnknownObjectOffset(raw))
    }
}

fn oop_prefix_word_count(format: Format, payload_words: &[u64]) -> usize {
    match format {
        Format::FixedPointers | Format::VarPointers | Format::FixedAndVar | Format::Weak => {
            payload_words.len()
        }
        Format::CompiledMethod => MethodHeaderFields::decode(Oop::from_raw(payload_words[0]))
            .map(|header| 1 + header.num_literals as usize)
            .unwrap_or(0),
        _ => 0,
    }
}

fn snapshots_with_offsets(heap: &Heap) -> Vec<SerializedObject> {
    let mut offset = 8u64;
    let mut out = Vec::new();
    for snapshot in heap.snapshots() {
        out.push(SerializedObject { offset, snapshot });
        offset += object_size_bytes(&out.last().unwrap().snapshot);
    }
    out
}

pub fn save_vm<P: AsRef<Path>>(vm: &Vm, path: P) -> Result<(), ImageError> {
    let snapshots = snapshots_with_offsets(&vm.heap);
    let offset_by_raw = snapshots
        .iter()
        .map(|entry| (entry.snapshot.oop.raw(), entry.offset))
        .collect::<HashMap<_, _>>();

    let mut file = File::create(path)?;
    file.write_all(MAGIC)?;
    write_u32(&mut file, VERSION)?;

    write_u64(&mut file, snapshots.len() as u64)?;
    for entry in &snapshots {
        write_u64(&mut file, entry.offset)?;
        write_u64(&mut file, entry.snapshot.header_raw)?;
        write_u8(
            &mut file,
            match entry.snapshot.generation {
                Generation::Young => 0,
                Generation::Old => 1,
            },
        )?;
        write_u64(&mut file, entry.snapshot.slot_count as u64)?;
        write_u64(
            &mut file,
            entry
                .snapshot
                .byte_len
                .map(|len| len as u64)
                .unwrap_or(NONE_U64),
        )?;
        write_u64(&mut file, entry.snapshot.payload_words.len() as u64)?;
        let oop_words = oop_prefix_word_count(entry.snapshot.format, &entry.snapshot.payload_words);
        for (index, word) in entry.snapshot.payload_words.iter().copied().enumerate() {
            let encoded = if index < oop_words {
                encode_oop(Oop::from_raw(word), &offset_by_raw)?
            } else {
                word
            };
            write_u64(&mut file, encoded)?;
        }
    }

    write_u64(&mut file, vm.class_table.len() as u64)?;
    for index in 0..vm.class_table.len() as u32 {
        if let Some(info) = vm.class_table.get(index) {
            write_u8(&mut file, 1)?;
            write_u64(&mut file, encode_oop(info.oop, &offset_by_raw)?)?;
            write_string(&mut file, &info.name)?;
            write_u64(
                &mut file,
                info.superclass.map(|v| v as u64).unwrap_or(NONE_U64),
            )?;
            write_u8(&mut file, info.instance_format as u8)?;
            write_u64(&mut file, info.fixed_fields as u64)?;
            write_u64(&mut file, info.instance_variables.len() as u64)?;
            for name in &info.instance_variables {
                write_string(&mut file, name)?;
            }
            write_u64(&mut file, info.methods.len() as u64)?;
            for (selector, method) in &info.methods {
                write_u64(&mut file, encode_oop(*selector, &offset_by_raw)?)?;
                write_u64(&mut file, encode_oop(*method, &offset_by_raw)?)?;
            }
        } else {
            write_u8(&mut file, 0)?;
        }
    }

    write_u64(&mut file, vm.special_objects.len() as u64)?;
    for oop in &vm.special_objects {
        write_u64(&mut file, encode_oop(*oop, &offset_by_raw)?)?;
    }

    write_u64(&mut file, vm.special_selectors.len() as u64)?;
    for oop in &vm.special_selectors {
        write_u64(&mut file, encode_oop(*oop, &offset_by_raw)?)?;
    }

    write_u64(&mut file, vm.symbols.len() as u64)?;
    for (name, oop) in &vm.symbols {
        write_string(&mut file, name)?;
        write_u64(&mut file, encode_oop(*oop, &offset_by_raw)?)?;
    }

    write_u64(&mut file, vm.globals.len() as u64)?;
    for (name, oop) in &vm.globals {
        write_string(&mut file, name)?;
        write_u64(&mut file, encode_oop(*oop, &offset_by_raw)?)?;
    }

    write_u64(&mut file, vm.method_sources.len() as u64)?;
    for (method, source) in &vm.method_sources {
        write_u64(&mut file, encode_oop(*method, &offset_by_raw)?)?;
        write_string(&mut file, source)?;
    }

    Ok(())
}

pub fn load_vm<P: AsRef<Path>>(path: P) -> Result<Vm, ImageError> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(ImageError::InvalidFormat("bad magic"));
    }
    let version = read_u32(&mut file)?;
    if version != VERSION {
        return Err(ImageError::InvalidVersion(version));
    }

    let object_count = read_u64(&mut file)? as usize;
    let mut serialized_objects = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        let offset = read_u64(&mut file)?;
        let header_raw = read_u64(&mut file)?;
        let generation = match read_u8(&mut file)? {
            0 => Generation::Young,
            1 => Generation::Old,
            _ => return Err(ImageError::InvalidFormat("bad generation")),
        };
        let slot_count = read_u64(&mut file)? as usize;
        let byte_len = match read_u64(&mut file)? {
            NONE_U64 => None,
            value => Some(value as usize),
        };
        let payload_len = read_u64(&mut file)? as usize;
        let mut payload_words = Vec::with_capacity(payload_len);
        for _ in 0..payload_len {
            payload_words.push(read_u64(&mut file)?);
        }
        if payload_len != slot_count {
            return Err(ImageError::InvalidFormat("payload length mismatch"));
        }
        serialized_objects.push((
            offset,
            LoadedObject {
                header_raw,
                slot_count,
                generation,
                byte_len,
                payload_words,
            },
        ));
    }

    let mut heap = Heap::new();
    let mut offset_to_oop = HashMap::new();
    let mut allocated = Vec::with_capacity(serialized_objects.len());
    for (offset, object) in &serialized_objects {
        let oop = heap.allocate_from_raw_parts(
            object.header_raw,
            object.slot_count,
            object.generation,
            Some(&object.payload_words),
            object.byte_len,
        );
        offset_to_oop.insert(*offset, oop);
        allocated.push((*offset, oop, object.clone()));
    }

    for (_, oop, object) in &allocated {
        let format = HeaderWord::from_raw(object.header_raw).format();
        let oop_words = oop_prefix_word_count(format, &object.payload_words);
        for (index, serialized_word) in object
            .payload_words
            .iter()
            .copied()
            .enumerate()
            .take(oop_words)
        {
            let decoded = decode_oop(serialized_word, &offset_to_oop)?;
            heap.write_word(*oop, index, decoded.raw());
        }
        if let Some(byte_len) = object.byte_len {
            heap.set_byte_len(*oop, byte_len);
        }
    }

    let class_table_len = read_u64(&mut file)? as usize;
    let mut class_table = crate::class_table::ClassTable::new();
    if class_table_len > 0 {
        for index in 0..class_table_len as u32 {
            let present = read_u8(&mut file)?;
            if present == 0 {
                continue;
            }
            let oop = decode_oop(read_u64(&mut file)?, &offset_to_oop)?;
            let name = read_string(&mut file)?;
            let superclass = match read_u64(&mut file)? {
                NONE_U64 => None,
                value => Some(value as u32),
            };
            let format = Format::from_u8(read_u8(&mut file)?)
                .ok_or(ImageError::InvalidFormat("bad class format"))?;
            let fixed_fields = read_u64(&mut file)? as usize;
            let instance_variable_count = read_u64(&mut file)? as usize;
            let mut instance_variables = Vec::with_capacity(instance_variable_count);
            for _ in 0..instance_variable_count {
                instance_variables.push(read_string(&mut file)?);
            }
            let method_count = read_u64(&mut file)? as usize;
            let mut methods = HashMap::with_capacity(method_count);
            for _ in 0..method_count {
                let selector = decode_oop(read_u64(&mut file)?, &offset_to_oop)?;
                let method = decode_oop(read_u64(&mut file)?, &offset_to_oop)?;
                methods.insert(selector, method);
            }
            class_table.insert_at(
                index,
                ClassInfo {
                    oop,
                    name,
                    superclass,
                    instance_format: format,
                    fixed_fields,
                    instance_variables,
                    methods,
                },
            );
        }
    }

    let special_object_count = read_u64(&mut file)? as usize;
    let mut special_objects = Vec::with_capacity(special_object_count);
    for _ in 0..special_object_count {
        special_objects.push(decode_oop(read_u64(&mut file)?, &offset_to_oop)?);
    }

    let special_selector_count = read_u64(&mut file)? as usize;
    let mut special_selectors = Vec::with_capacity(special_selector_count);
    for _ in 0..special_selector_count {
        special_selectors.push(decode_oop(read_u64(&mut file)?, &offset_to_oop)?);
    }

    let symbol_count = read_u64(&mut file)? as usize;
    let mut symbols = HashMap::with_capacity(symbol_count);
    for _ in 0..symbol_count {
        let name = read_string(&mut file)?;
        let oop = decode_oop(read_u64(&mut file)?, &offset_to_oop)?;
        symbols.insert(name, oop);
    }

    let global_count = read_u64(&mut file)? as usize;
    let mut globals = HashMap::with_capacity(global_count);
    for _ in 0..global_count {
        let name = read_string(&mut file)?;
        let oop = decode_oop(read_u64(&mut file)?, &offset_to_oop)?;
        globals.insert(name, oop);
    }

    let method_source_count = read_u64(&mut file)? as usize;
    let mut method_sources = HashMap::with_capacity(method_source_count);
    for _ in 0..method_source_count {
        let method = decode_oop(read_u64(&mut file)?, &offset_to_oop)?;
        let source = read_string(&mut file)?;
        method_sources.insert(method, source);
    }

    Ok(Vm::from_parts(
        heap,
        class_table,
        special_objects,
        special_selectors,
        symbols,
        globals,
        method_sources,
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        bytecode::{PUSH_ONE, PUSH_TWO, RETURN_TOP, SEND_SPECIAL_BASE},
        compiler::compile_method_source,
        load_source,
        object::MethodHeaderFields,
        value::Oop,
    };

    use super::{load_vm, save_vm};
    use crate::interpreter::Vm;

    #[test]
    fn image_roundtrip_preserves_bootstrap() {
        let mut path = std::env::temp_dir();
        path.push(format!("smallesttalk-test-{}.img", std::process::id()));

        let vm = Vm::new();
        save_vm(&vm, &path).unwrap();
        let loaded = load_vm(&path).unwrap();
        assert_eq!(loaded.class_table.len(), vm.class_table.len());
        assert_eq!(loaded.special_objects.len(), vm.special_objects.len());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn image_roundtrip_preserves_methods() {
        let mut path = PathBuf::from(std::env::temp_dir());
        path.push(format!("smallesttalk-method-{}.img", std::process::id()));

        let mut vm = Vm::new();
        let method = vm.compiled_method(
            MethodHeaderFields {
                num_args: 0,
                num_temps: 0,
                num_literals: 0,
                flags: 0,
            },
            &[],
            &[PUSH_ONE, PUSH_TWO, SEND_SPECIAL_BASE, RETURN_TOP],
        );
        let result = vm.run_method(method, vm.true_oop(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
        save_vm(&vm, &path).unwrap();
        let loaded = load_vm(&path).unwrap();
        assert_eq!(loaded.class_table.len(), vm.class_table.len());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn image_roundtrip_preserves_globals_and_compiled_source_methods() {
        let mut path = PathBuf::from(std::env::temp_dir());
        path.push(format!("smallesttalk-globals-{}.img", std::process::id()));

        let mut vm = Vm::new();
        vm.set_global("Answer", Oop::from_i64(42).unwrap());
        compile_method_source(&mut vm, crate::class_table::CLASS_INDEX_TRUE, "answer ^ Answer").unwrap();
        save_vm(&vm, &path).unwrap();
        let mut loaded = load_vm(&path).unwrap();
        assert_eq!(loaded.global_value("Answer").and_then(Oop::as_i64), Some(42));
        let selector = loaded.intern_symbol("answer");
        let result = loaded.send(loaded.true_oop(), selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(42));
        let (_, method) = loaded
            .lookup_method(crate::class_table::CLASS_INDEX_TRUE, selector)
            .unwrap();
        assert_eq!(loaded.method_source(method), Some("answer ^ Answer"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn image_roundtrip_preserves_source_loaded_classes() {
        let mut path = PathBuf::from(std::env::temp_dir());
        path.push(format!("smallesttalk-source-{}.img", std::process::id()));

        let mut vm = Vm::new();
        load_source(
            &mut vm,
            "doit\nBehavior subclass: #Point instanceVariableNames: 'x y'\n!\nPoint >>\nx\n    ^ x\n!\nPoint >>\nx: value\n    x := value\n!\ndoit\nP := Point new.\nP x: 64.\n!\n",
        )
        .unwrap();
        save_vm(&vm, &path).unwrap();
        let mut loaded = load_vm(&path).unwrap();
        let point = loaded.global_value("P").unwrap();
        let selector = loaded.intern_symbol("x");
        let result = loaded.send(point, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(64));
        let _ = std::fs::remove_file(path);
    }
}
