use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    fmt,
    time::{Duration, Instant},
};

use crate::{
    bootstrap, corelib, gui_snapshot,
    bytecode::{
        BLOCK_RETURN, DUP, EXTENDED_SEND, EXTENDED_SUPER_SEND, JUMP_BACK, JUMP_BACK_LONG,
        JUMP_FALSE, JUMP_FALSE_LONG, JUMP_FORWARD, JUMP_FORWARD_LONG, JUMP_TRUE, JUMP_TRUE_LONG,
        POP, POP_STORE_INST_VAR_BASE, POP_STORE_INST_VAR_EXT, POP_STORE_TEMP_BASE,
        POP_STORE_TEMP_EXT, PUSH_CLOSURE, PUSH_FALSE, PUSH_INST_VAR_BASE, PUSH_INST_VAR_EXT,
        PUSH_LIT_VAR_BASE, PUSH_LIT_VAR_EXT, PUSH_LITERAL_BASE, PUSH_LITERAL_EXT, PUSH_MINUS_ONE,
        PUSH_NEW_ARRAY, PUSH_NIL, PUSH_ONE, PUSH_SELF, PUSH_SMALL_INT_EXT, PUSH_TEMP_BASE,
        PUSH_TEMP_EXT, PUSH_TRUE, PUSH_TWO, PUSH_ZERO, RETURN_NIL, RETURN_SELF, RETURN_TOP,
        SEND_EXT, SEND_SHORT_BASE, SEND_SPECIAL_BASE, SUPER_SEND_EXT, selector_arity,
    },
    class_table::{
        CLASS_INDEX_ARRAY, CLASS_INDEX_ASSOCIATION, CLASS_INDEX_BEHAVIOR,
        CLASS_INDEX_BLOCK_CLOSURE, CLASS_INDEX_BYTE_ARRAY, CLASS_INDEX_COMPILED_METHOD,
        CLASS_INDEX_MESSAGE, CLASS_INDEX_METHOD_CONTEXT, CLASS_INDEX_SMALL_INTEGER,
        CLASS_INDEX_STRING, CLASS_INDEX_SYMBOL, CLASS_INDEX_UNDEFINED_OBJECT, ClassInfo,
        ClassTable,
    },
    heap::{GcKind, GcResult, Generation, Heap},
    method_cache::MethodCache,
    object::{Format, MethodHeaderFields},
    primitives::{
        PRIMITIVE_AT, PRIMITIVE_AT_PUT, PRIMITIVE_BASIC_NEW, PRIMITIVE_BASIC_NEW_SIZED,
        PRIMITIVE_CLASS, PRIMITIVE_COMPILED_METHOD, PRIMITIVE_COPY_FROM_TO, PRIMITIVE_EQUALS,
        PRIMITIVE_GLOBAL_ASSOCIATION, PRIMITIVE_HOST_DISPLAY_OPEN,
        PRIMITIVE_HOST_DISPLAY_PRESENT_FORM, PRIMITIVE_HOST_DISPLAY_SAVE_PNG,
        PRIMITIVE_HOST_NEXT_EVENT,
        PRIMITIVE_INSTALL_COMPILED_METHOD, PRIMITIVE_INSTALL_METHOD,
        PRIMITIVE_FORM_COPY_RECTANGLE, PRIMITIVE_FORM_FILL_RECTANGLE,
        PRIMITIVE_INSTANCE_VARIABLE_INDEX,
        PRIMITIVE_INTERN_SYMBOL, PRIMITIVE_MILLISECOND_CLOCK, PRIMITIVE_SIZE,
        PRIMITIVE_SLEEP_MILLISECONDS, PRIMITIVE_SUBCLASS,
        PRIMITIVE_SUBCLASS_EXTENDED, PRIMITIVE_THIS_CONTEXT,
    },
    value::Oop,
};

#[derive(Debug, Clone)]
pub enum VmError {
    StackUnderflow,
    InvalidMethod(Oop),
    InvalidOpcode { method: Oop, ip: usize, opcode: u8 },
    InvalidClassIndex(u32),
    WrongArgumentCount { expected: usize, actual: usize },
    MessageNotUnderstood { class_index: u32, selector: String },
    TypeError(&'static str),
    IndexOutOfBounds { index: usize, size: usize },
    CannotReturn,
    PrimitiveFailed(u16),
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StackUnderflow => write!(f, "stack underflow"),
            Self::InvalidMethod(method) => write!(f, "invalid compiled method: {method:?}"),
            Self::InvalidOpcode { method, ip, opcode } => {
                write!(f, "invalid opcode 0x{opcode:02x} at {ip} in {method:?}")
            }
            Self::InvalidClassIndex(index) => write!(f, "invalid class index {index}"),
            Self::WrongArgumentCount { expected, actual } => {
                write!(f, "wrong argument count: expected {expected}, got {actual}")
            }
            Self::MessageNotUnderstood {
                class_index,
                selector,
            } => {
                write!(f, "class {class_index} does not understand #{selector}")
            }
            Self::TypeError(msg) => write!(f, "type error: {msg}"),
            Self::IndexOutOfBounds { index, size } => {
                write!(f, "index {index} out of bounds for size {size}")
            }
            Self::CannotReturn => write!(f, "cannot perform non-local return"),
            Self::PrimitiveFailed(index) => write!(f, "primitive {index} failed"),
        }
    }
}

impl Error for VmError {}

pub struct VmStack {
    slots: Vec<Oop>,
    pub sp: usize,
    pub fp: usize,
}

impl Default for VmStack {
    fn default() -> Self {
        Self::new()
    }
}

impl VmStack {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            sp: 0,
            fp: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.sp
    }

    pub fn is_empty(&self) -> bool {
        self.sp == 0
    }

    pub fn push(&mut self, value: Oop) {
        if self.sp == self.slots.len() {
            self.slots.push(value);
        } else {
            self.slots[self.sp] = value;
        }
        self.sp += 1;
    }

    pub fn pop(&mut self) -> Result<Oop, VmError> {
        if self.sp == 0 {
            return Err(VmError::StackUnderflow);
        }
        self.sp -= 1;
        Ok(self.slots[self.sp])
    }

    pub fn peek(&self) -> Result<Oop, VmError> {
        self.sp
            .checked_sub(1)
            .and_then(|index| self.slots.get(index).copied())
            .ok_or(VmError::StackUnderflow)
    }

    pub fn get(&self, index: usize) -> Result<Oop, VmError> {
        self.slots
            .get(index)
            .copied()
            .ok_or(VmError::StackUnderflow)
    }

    pub fn set(&mut self, index: usize, value: Oop) -> Result<(), VmError> {
        let slot = self.slots.get_mut(index).ok_or(VmError::StackUnderflow)?;
        *slot = value;
        Ok(())
    }

    pub fn truncate(&mut self, len: usize) {
        self.sp = len;
    }
}

#[derive(Clone, Copy, Debug)]
struct ClosureHome {
    home_frame_id: usize,
    receiver: Oop,
}

#[derive(Clone, Copy, Debug)]
enum ExecOutcome {
    Returned(Oop),
    NonLocalReturn { home_frame_id: usize, result: Oop },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostEvent {
    MouseMove { x: i64, y: i64 },
    MouseDown { x: i64, y: i64, button: i64 },
    MouseUp { x: i64, y: i64, button: i64 },
    KeyDown { key: i64 },
    KeyUp { key: i64 },
    Resize { width: i64, height: i64 },
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDisplaySnapshot {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub presents: usize,
    pub last_frame: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
struct HostDisplayState {
    width: usize,
    height: usize,
    depth: usize,
    presents: usize,
    last_frame: Vec<u8>,
}

pub struct Vm {
    pub heap: Heap,
    pub stack: VmStack,
    pub class_table: ClassTable,
    pub method_cache: MethodCache,
    pub special_objects: Vec<Oop>,
    pub special_selectors: Vec<Oop>,
    pub symbols: HashMap<String, Oop>,
    pub globals: HashMap<String, Oop>,
    method_owners: HashMap<Oop, u32>,
    closure_homes: HashMap<Oop, ClosureHome>,
    active_frame_ids: Vec<usize>,
    next_frame_id: usize,
    host_displays: HashMap<u32, HostDisplayState>,
    next_host_display_id: u32,
    host_events: VecDeque<HostEvent>,
    clock_start: Instant,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub(crate) fn from_parts(
        mut heap: Heap,
        mut class_table: ClassTable,
        special_objects: Vec<Oop>,
        special_selectors: Vec<Oop>,
        mut symbols: HashMap<String, Oop>,
        globals: HashMap<String, Oop>,
    ) -> Self {
        bootstrap::install_bootstrap_methods(&mut heap, &mut class_table, &mut symbols);
        let mut vm = Self {
            heap,
            stack: VmStack::new(),
            class_table,
            method_cache: MethodCache::new(),
            special_objects,
            special_selectors,
            symbols,
            globals,
            method_owners: HashMap::new(),
            closure_homes: HashMap::new(),
            active_frame_ids: Vec::new(),
            next_frame_id: 1,
            host_displays: HashMap::new(),
            next_host_display_id: 1,
            host_events: VecDeque::new(),
            clock_start: Instant::now(),
        };
        vm.rebuild_runtime_metadata();
        vm.sync_heap_metadata();
        crate::load_source(&mut vm, corelib::SOURCE).expect("core library source must load");
        vm.install_gui_runtime_methods_if_available()
            .expect("gui runtime methods must install");
        vm
    }

    pub fn new() -> Self {
        let state = bootstrap::build();
        Self::from_parts(
            state.heap,
            state.class_table,
            state.special_objects,
            state.special_selectors,
            state.symbols,
            state.globals,
        )
    }

    fn class_index_by_name(&self, name: &str) -> Option<u32> {
        self.class_table
            .iter()
            .find_map(|(index, info)| (info.name == name).then_some(index))
    }

    pub(crate) fn install_gui_runtime_methods_if_available(&mut self) -> Result<(), VmError> {
        let behavior = crate::class_table::CLASS_INDEX_BEHAVIOR;
        self.install_primitive_method(behavior, "hostDisplayOpenWidth:height:depth:", 3, PRIMITIVE_HOST_DISPLAY_OPEN)?;
        self.install_primitive_method(behavior, "hostNextEvent", 0, PRIMITIVE_HOST_NEXT_EVENT)?;
        self.install_primitive_method(behavior, "millisecondClock", 0, PRIMITIVE_MILLISECOND_CLOCK)?;
        self.install_primitive_method(behavior, "sleepMilliseconds:", 1, PRIMITIVE_SLEEP_MILLISECONDS)?;
        if let Some(host_display) = self.class_index_by_name("HostDisplay") {
            self.install_primitive_method(host_display, "presentForm:", 1, PRIMITIVE_HOST_DISPLAY_PRESENT_FORM)?;
            self.install_primitive_method(host_display, "savePNG:", 1, PRIMITIVE_HOST_DISPLAY_SAVE_PNG)?;
        }
        if let Some(form) = self.class_index_by_name("Form") {
            self.install_primitive_method(form, "fillRectangleX:y:width:height:with:", 5, PRIMITIVE_FORM_FILL_RECTANGLE)?;
            self.install_primitive_method(
                form,
                "copyRectangleX:y:width:height:from:atX:y:",
                7,
                PRIMITIVE_FORM_COPY_RECTANGLE,
            )?;
        }
        Ok(())
    }

    fn install_primitive_method(
        &mut self,
        class_index: u32,
        selector_text: &str,
        num_args: u8,
        primitive_index: u16,
    ) -> Result<(), VmError> {
        let selector = self.intern_symbol(selector_text);
        let method = self.compiled_method(
            MethodHeaderFields {
                num_args,
                num_temps: 0,
                num_literals: 0,
                flags: primitive_index as u32,
            },
            &[],
            &[RETURN_TOP],
        );
        self.add_method(class_index, selector, method)
    }

    pub fn enqueue_host_event(&mut self, event: HostEvent) {
        self.host_events.push_back(event);
    }

    pub fn host_display_snapshot(&self, handle: u32) -> Option<HostDisplaySnapshot> {
        let state = self.host_displays.get(&handle)?;
        Some(HostDisplaySnapshot {
            width: state.width,
            height: state.height,
            depth: state.depth,
            presents: state.presents,
            last_frame: state.last_frame.clone(),
        })
    }

    pub fn special_object(&self, index: usize) -> Option<Oop> {
        self.special_objects.get(index).copied()
    }

    pub fn true_oop(&self) -> Oop {
        self.special_objects[bootstrap::SPECIAL_OBJECT_TRUE]
    }

    pub fn false_oop(&self) -> Oop {
        self.special_objects[bootstrap::SPECIAL_OBJECT_FALSE]
    }

    pub fn boolean_oop(&self, value: bool) -> Oop {
        if value {
            self.true_oop()
        } else {
            self.false_oop()
        }
    }

    pub fn class_of(&self, oop: Oop) -> Result<u32, VmError> {
        if oop.is_nil() {
            Ok(CLASS_INDEX_UNDEFINED_OBJECT)
        } else if oop.is_small_int() {
            Ok(CLASS_INDEX_SMALL_INTEGER)
        } else {
            self.heap
                .header(oop)
                .map(|header| unsafe { header.class_index() })
                .ok_or(VmError::TypeError("expected object"))
        }
    }

    pub fn symbol_text(&self, symbol: Oop) -> Result<String, VmError> {
        let class_index = self.class_of(symbol)?;
        if class_index != CLASS_INDEX_SYMBOL && class_index != CLASS_INDEX_STRING {
            return Err(VmError::TypeError("expected Symbol or String"));
        }
        let bytes = self
            .heap
            .bytes(symbol)
            .ok_or(VmError::TypeError("expected byte object"))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn intern_symbol(&mut self, text: &str) -> Oop {
        if let Some(existing) = self.symbols.get(text) {
            return *existing;
        }
        let oop = self
            .heap
            .allocate_bytes_in(CLASS_INDEX_SYMBOL, text.as_bytes(), Generation::Old);
        self.symbols.insert(text.to_string(), oop);
        self.refresh_symbol_table_object();
        oop
    }

    fn refresh_symbol_table_object(&mut self) {
        if self.special_objects.len() <= bootstrap::SPECIAL_OBJECT_SYMBOL_TABLE {
            return;
        }
        let values = self.symbols.values().copied().collect::<Vec<_>>();
        let table = self.make_array(&values);
        self.special_objects[bootstrap::SPECIAL_OBJECT_SYMBOL_TABLE] = table;
    }

    fn refresh_class_table_array_object(&mut self) {
        if self.special_objects.len() <= bootstrap::SPECIAL_OBJECT_CLASS_TABLE_ARRAY {
            return;
        }
        let values = (1..self.class_table.len() as u32)
            .filter_map(|index| self.class_table.class_oop(index))
            .collect::<Vec<_>>();
        let table = self.make_array(&values);
        self.special_objects[bootstrap::SPECIAL_OBJECT_CLASS_TABLE_ARRAY] = table;
    }

    pub fn new_class(
        &mut self,
        name: &str,
        superclass: Option<u32>,
        format: Format,
        fixed_fields: usize,
    ) -> Result<u32, VmError> {
        let inherited_fixed_fields = if let Some(superclass_index) = superclass {
            if self.class_table.get(superclass_index).is_none() {
                return Err(VmError::InvalidClassIndex(superclass_index));
            }
            if superclass_index == CLASS_INDEX_BEHAVIOR {
                0
            } else {
                self.class_table
                    .get(superclass_index)
                    .ok_or(VmError::InvalidClassIndex(superclass_index))?
                    .fixed_fields
            }
        } else {
            0
        };
        let total_fixed_fields = inherited_fixed_fields + fixed_fields;
        let class_oop = self.heap.allocate_object_in(
            CLASS_INDEX_BEHAVIOR,
            Format::FixedPointers,
            6,
            Generation::Old,
        );
        let index = self.class_table.add_class(ClassInfo {
            oop: class_oop,
            name: name.to_string(),
            superclass,
            instance_format: format,
            fixed_fields: total_fixed_fields,
            instance_variables: Vec::new(),
            methods: HashMap::new(),
        });
        let name_symbol = self.intern_symbol(name);
        let empty_array = self.heap.allocate_object_in(
            CLASS_INDEX_ARRAY,
            Format::VarPointers,
            0,
            Generation::Old,
        );
        let method_dict = self.heap.allocate_object_in(
            crate::class_table::CLASS_INDEX_METHOD_DICTIONARY,
            Format::VarPointers,
            0,
            Generation::Old,
        );
        self.heap.write_slot(
            class_oop,
            0,
            superclass
                .and_then(|superclass_index| self.class_table.class_oop(superclass_index))
                .unwrap_or_else(Oop::nil),
        );
        self.heap.write_slot(class_oop, 1, method_dict);
        self.heap.write_slot(
            class_oop,
            2,
            crate::class_table::encode_format_descriptor(format, total_fixed_fields),
        );
        self.heap.write_slot(class_oop, 3, empty_array);
        self.heap.write_slot(class_oop, 4, name_symbol);
        self.heap.write_slot(class_oop, 5, empty_array);
        self.sync_heap_metadata();
        Ok(index)
    }

    pub fn set_instance_variables(
        &mut self,
        class_index: u32,
        names: Vec<String>,
    ) -> Result<(), VmError> {
        let class_oop = self
            .class_table
            .class_oop(class_index)
            .ok_or(VmError::InvalidClassIndex(class_index))?;
        let ivar_symbols = names
            .iter()
            .map(|name| self.intern_symbol(name))
            .collect::<Vec<_>>();
        let ivar_array = self.make_array(&ivar_symbols);
        if let Some(info) = self.class_table.get_mut(class_index) {
            info.instance_variables = names;
        }
        self.heap.write_slot(class_oop, 3, ivar_array);
        self.sync_heap_metadata();
        Ok(())
    }

    pub fn add_method(
        &mut self,
        class_index: u32,
        selector: Oop,
        method: Oop,
    ) -> Result<(), VmError> {
        if self.class_table.get(class_index).is_none() {
            return Err(VmError::InvalidClassIndex(class_index));
        }
        self.class_table.set_method(class_index, selector, method);
        self.method_owners.insert(method, class_index);
        self.method_cache.clear();
        self.sync_heap_metadata();
        Ok(())
    }

    pub fn owner_class_of_method(&self, method: Oop) -> Option<u32> {
        self.method_owners.get(&method).copied()
    }

    pub fn compiled_method(
        &mut self,
        header: MethodHeaderFields,
        literals: &[Oop],
        bytecodes: &[u8],
    ) -> Oop {
        self.heap.allocate_compiled_method_in(
            CLASS_INDEX_COMPILED_METHOD,
            header,
            literals,
            bytecodes,
            Generation::Old,
        )
    }

    pub fn new_instance(
        &mut self,
        class_index: u32,
        variable_slots: usize,
    ) -> Result<Oop, VmError> {
        let info = self
            .class_table
            .get(class_index)
            .ok_or(VmError::InvalidClassIndex(class_index))?
            .clone();
        let slot_count = match info.instance_format {
            Format::Empty => 0,
            Format::FixedPointers => info.fixed_fields,
            Format::VarPointers => variable_slots,
            Format::FixedAndVar | Format::Weak => info.fixed_fields + variable_slots,
            Format::Words => info.fixed_fields.max(variable_slots),
            Format::Bytes8
            | Format::Bytes16
            | Format::Bytes24
            | Format::Bytes32
            | Format::CompiledMethod => {
                return Err(VmError::TypeError(
                    "use byte/compiled method allocators for raw objects",
                ));
            }
        };
        Ok(self
            .heap
            .allocate_object(class_index, info.instance_format, slot_count))
    }

    pub fn make_array(&mut self, values: &[Oop]) -> Oop {
        let array = self
            .heap
            .allocate_object(CLASS_INDEX_ARRAY, Format::VarPointers, values.len());
        for (index, value) in values.iter().copied().enumerate() {
            let _ = self.heap.write_slot(array, index, value);
        }
        array
    }

    fn rebuild_runtime_metadata(&mut self) {
        self.method_owners.clear();
        for (class_index, info) in self.class_table.iter() {
            for method in info.methods.values().copied() {
                self.method_owners.insert(method, class_index);
            }
        }
        self.method_cache.clear();
    }

    fn build_method_dictionary(&mut self, methods: &HashMap<Oop, Oop>) -> Oop {
        let dict = self.heap.allocate_object_in(
            crate::class_table::CLASS_INDEX_METHOD_DICTIONARY,
            Format::VarPointers,
            methods.len() * 2,
            Generation::Old,
        );
        for (entry_index, (selector, method)) in methods.iter().enumerate() {
            let slot = entry_index * 2;
            let _ = self.heap.write_slot(dict, slot, *selector);
            let _ = self.heap.write_slot(dict, slot + 1, *method);
        }
        dict
    }

    fn refresh_subclass_links(&mut self) {
        let subclass_map = self
            .class_table
            .iter()
            .filter_map(|(class_index, info)| {
                info.superclass.map(|superclass| (superclass, class_index))
            })
            .fold(
                HashMap::<u32, Vec<u32>>::new(),
                |mut map, (superclass, subclass)| {
                    map.entry(superclass).or_default().push(subclass);
                    map
                },
            );

        let class_entries = self
            .class_table
            .iter()
            .map(|(index, info)| (index, info.oop))
            .collect::<Vec<_>>();
        for (class_index, class_oop) in class_entries {
            let subclass_oops = subclass_map
                .get(&class_index)
                .into_iter()
                .flatten()
                .filter_map(|subclass_index| self.class_table.class_oop(*subclass_index))
                .collect::<Vec<_>>();
            let subclasses_array = self.make_array(&subclass_oops);
            let _ = self.heap.write_slot(class_oop, 5, subclasses_array);
        }
    }

    fn refresh_method_dictionary_objects(&mut self) {
        let class_entries = self
            .class_table
            .iter()
            .map(|(index, info)| (index, info.oop, info.methods.clone()))
            .collect::<Vec<_>>();
        for (_class_index, class_oop, methods) in class_entries {
            let dict = self.build_method_dictionary(&methods);
            let _ = self.heap.write_slot(class_oop, 1, dict);
        }
    }

    fn build_association(&mut self, key: Oop, value: Oop) -> Oop {
        let association = self.heap.allocate_object_in(
            CLASS_INDEX_ASSOCIATION,
            Format::FixedPointers,
            2,
            Generation::Old,
        );
        let _ = self.heap.write_slot(association, 0, key);
        let _ = self.heap.write_slot(association, 1, value);
        association
    }

    fn refresh_smalltalk_object(&mut self) {
        if self.special_objects.len() <= bootstrap::SPECIAL_OBJECT_SMALLTALK {
            return;
        }
        let entries = self
            .class_table
            .iter()
            .map(|(_, info)| (info.name.clone(), info.oop))
            .collect::<Vec<_>>();
        let mut associations = Vec::with_capacity(entries.len() + self.globals.len());
        for (name, class_oop) in entries {
            let key = self.intern_symbol(&name);
            associations.push(self.build_association(key, class_oop));
        }
        associations.extend(self.globals.values().copied());
        let dictionary = self.make_array(&associations);
        self.special_objects[bootstrap::SPECIAL_OBJECT_SMALLTALK] = dictionary;
    }

    pub fn global_association(&mut self, name: &str) -> Oop {
        if let Some(existing) = self.globals.get(name) {
            return *existing;
        }
        let key = self.intern_symbol(name);
        let initial_value = self
            .class_table
            .iter()
            .find_map(|(_, info)| (info.name == name).then_some(info.oop))
            .unwrap_or_else(Oop::nil);
        let association = self.build_association(key, initial_value);
        self.globals.insert(name.to_string(), association);
        self.refresh_smalltalk_object();
        association
    }

    pub fn set_global(&mut self, name: &str, value: Oop) {
        let association = self.global_association(name);
        let _ = self.heap.write_slot(association, 1, value);
        self.refresh_smalltalk_object();
    }

    pub fn global_value(&self, name: &str) -> Option<Oop> {
        let association = self.globals.get(name).copied()?;
        self.heap.read_slot(association, 1)
    }

    fn sync_heap_metadata(&mut self) {
        self.refresh_method_dictionary_objects();
        self.refresh_subclass_links();
        self.refresh_class_table_array_object();
        self.refresh_symbol_table_object();
        self.refresh_smalltalk_object();
    }

    fn remap_oop(relocated: &std::collections::HashMap<u64, Oop>, oop: &mut Oop) {
        if !oop.is_heap_ptr() {
            return;
        }
        *oop = relocated.get(&oop.raw()).copied().unwrap_or_else(Oop::nil);
    }

    fn collect_roots(&self) -> Vec<Oop> {
        let mut roots = Vec::new();
        roots.extend_from_slice(&self.stack.slots[..self.stack.sp]);
        roots.extend(self.special_objects.iter().copied());
        roots.extend(self.special_selectors.iter().copied());
        roots.extend(self.symbols.values().copied());
        roots.extend(self.globals.values().copied());
        for (_, info) in self.class_table.iter() {
            roots.push(info.oop);
            roots.extend(info.methods.keys().copied());
            roots.extend(info.methods.values().copied());
        }
        roots.extend(self.method_owners.keys().copied());
        roots.extend(self.closure_homes.keys().copied());
        for home in self.closure_homes.values() {
            roots.push(home.receiver);
        }
        roots
    }

    fn apply_relocation(&mut self, relocated: &std::collections::HashMap<u64, Oop>) {
        for oop in &mut self.stack.slots[..self.stack.sp] {
            Self::remap_oop(relocated, oop);
        }
        for oop in &mut self.special_objects {
            Self::remap_oop(relocated, oop);
        }
        for oop in &mut self.special_selectors {
            Self::remap_oop(relocated, oop);
        }
        for oop in self.symbols.values_mut() {
            Self::remap_oop(relocated, oop);
        }
        for oop in self.globals.values_mut() {
            Self::remap_oop(relocated, oop);
        }
        for (_, info) in self.class_table.iter_mut() {
            Self::remap_oop(relocated, &mut info.oop);
            let old_methods = std::mem::take(&mut info.methods);
            let mut new_methods = HashMap::with_capacity(old_methods.len());
            for (mut selector, mut method) in old_methods {
                Self::remap_oop(relocated, &mut selector);
                Self::remap_oop(relocated, &mut method);
                if !selector.is_nil() && !method.is_nil() {
                    new_methods.insert(selector, method);
                }
            }
            info.methods = new_methods;
        }
        let old_homes = std::mem::take(&mut self.closure_homes);
        let mut new_homes = HashMap::with_capacity(old_homes.len());
        for (mut closure, mut home) in old_homes {
            Self::remap_oop(relocated, &mut closure);
            Self::remap_oop(relocated, &mut home.receiver);
            if !closure.is_nil() {
                new_homes.insert(closure, home);
            }
        }
        self.closure_homes = new_homes;
        self.rebuild_runtime_metadata();
    }

    pub fn minor_gc(&mut self) -> GcResult {
        let roots = self.collect_roots();
        let result = self.heap.collect_garbage(&roots, GcKind::Minor);
        self.apply_relocation(&result.relocated);
        result
    }

    pub fn full_gc(&mut self) -> GcResult {
        let roots = self.collect_roots();
        let result = self.heap.collect_garbage(&roots, GcKind::Full);
        self.apply_relocation(&result.relocated);
        result
    }

    pub fn make_message(&mut self, selector: Oop, args: &[Oop]) -> Oop {
        let message = self
            .heap
            .allocate_object(CLASS_INDEX_MESSAGE, Format::FixedPointers, 2);
        let args_array = self.make_array(args);
        let _ = self.heap.write_slot(message, 0, selector);
        let _ = self.heap.write_slot(message, 1, args_array);
        message
    }

    pub fn materialize_current_context(&mut self) -> Option<Oop> {
        if self.stack.sp == 0 || self.stack.fp + 3 >= self.stack.sp {
            return None;
        }
        let saved_fp = self.stack.get(self.stack.fp).ok()?;
        let saved_ip = self.stack.get(self.stack.fp + 1).ok()?;
        let method = self.stack.get(self.stack.fp + 2).ok()?;
        let receiver = self.stack.get(self.stack.fp + 3).ok()?;
        let frame_values = self.stack.slots[self.stack.fp + 4..self.stack.sp].to_vec();
        let frame_array = self.make_array(&frame_values);
        let context =
            self.heap
                .allocate_object(CLASS_INDEX_METHOD_CONTEXT, Format::FixedPointers, 6);
        let frame_id = self
            .active_frame_ids
            .last()
            .and_then(|id| Oop::from_i64(*id as i64))
            .unwrap_or_else(Oop::nil);
        let _ = self.heap.write_slot(context, 0, saved_fp);
        let _ = self.heap.write_slot(context, 1, saved_ip);
        let _ = self.heap.write_slot(context, 2, method);
        let _ = self.heap.write_slot(context, 3, receiver);
        let _ = self.heap.write_slot(context, 4, frame_id);
        let _ = self.heap.write_slot(context, 5, frame_array);
        Some(context)
    }

    fn lookup_method_in_dictionary(&self, class_index: u32, selector: Oop) -> Option<Oop> {
        let class_oop = self.class_table.class_oop(class_index)?;
        let method_dict = self.heap.read_slot(class_oop, 1)?;
        let slot_count = self.heap.slot_count(method_dict)?;
        let mut index = 0usize;
        while index + 1 < slot_count {
            if self.heap.read_slot(method_dict, index)? == selector {
                return self.heap.read_slot(method_dict, index + 1);
            }
            index += 2;
        }
        None
    }

    pub fn lookup_method(&mut self, class_index: u32, selector: Oop) -> Option<(u32, Oop)> {
        if let Some(method) = self.method_cache.lookup(class_index, selector) {
            let owner = self.owner_class_of_method(method)?;
            return Some((owner, method));
        }
        let mut current = Some(class_index);
        while let Some(owner_class) = current {
            if let Some(method) = self.lookup_method_in_dictionary(owner_class, selector) {
                self.method_cache.insert(class_index, selector, method);
                return Some((owner_class, method));
            }
            current = self.class_table.superclass_of(owner_class);
        }
        None
    }

    pub fn run_method(&mut self, method: Oop, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        let owner = self
            .owner_class_of_method(method)
            .unwrap_or(self.class_of(receiver)?);
        match self.execute_method_internal(owner, method, receiver, args, 0, None)? {
            ExecOutcome::Returned(value) => Ok(value),
            ExecOutcome::NonLocalReturn { .. } => Err(VmError::CannotReturn),
        }
    }

    pub fn send(&mut self, receiver: Oop, selector: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        match self.send_message(receiver, selector, args, None)? {
            ExecOutcome::Returned(value) => Ok(value),
            ExecOutcome::NonLocalReturn { .. } => Err(VmError::CannotReturn),
        }
    }

    fn send_message(
        &mut self,
        receiver: Oop,
        selector: Oop,
        args: &[Oop],
        super_lookup_class: Option<u32>,
    ) -> Result<ExecOutcome, VmError> {
        let selector_text = self.symbol_text(selector)?;
        let selector = self.intern_symbol(&selector_text);
        let receiver_class = self.class_of(receiver)?;

        if super_lookup_class.is_none()
            && receiver_class == CLASS_INDEX_BLOCK_CLOSURE
            && selector_text.starts_with("value")
        {
            let expected = self
                .heap
                .read_slot(receiver, 2)
                .and_then(Oop::as_i64)
                .unwrap_or(0) as usize;
            if expected == args.len() {
                return self.activate_closure(receiver, args);
            }
        }

        let start_class = super_lookup_class.unwrap_or(receiver_class);
        let Some((owner_class, method)) = self.lookup_method(start_class, selector) else {
            let dnu_selector = self.special_objects[bootstrap::SPECIAL_OBJECT_DNU_SELECTOR];
            if selector != dnu_selector {
                if let Some((dnu_owner, dnu_method)) =
                    self.lookup_method(receiver_class, dnu_selector)
                {
                    let message = self.make_message(selector, args);
                    return self.execute_method_internal(
                        dnu_owner,
                        dnu_method,
                        receiver,
                        &[message],
                        0,
                        None,
                    );
                }
            }
            return Err(VmError::MessageNotUnderstood {
                class_index: receiver_class,
                selector: selector_text.clone(),
            });
        };

        let header = self
            .heap
            .compiled_method_header(method)
            .ok_or(VmError::InvalidMethod(method))?;
        match header.primitive_index() {
            0 => self.execute_method_internal(owner_class, method, receiver, args, 0, None),
            PRIMITIVE_BASIC_NEW => {
                let class_index = self
                    .class_table
                    .class_index_of_oop(receiver)
                    .ok_or(VmError::TypeError("new expects a class receiver"))?;
                let result = self.new_instance(class_index, 0)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_BASIC_NEW_SIZED => {
                let class_index = self
                    .class_table
                    .class_index_of_oop(receiver)
                    .ok_or(VmError::TypeError("new: expects a class receiver"))?;
                if args.len() != 1 {
                    return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
                }
                let size = args[0]
                    .as_i64()
                    .ok_or(VmError::TypeError("new: expects SmallInteger size"))?
                    as usize;
                let info = self
                    .class_table
                    .get(class_index)
                    .ok_or(VmError::InvalidClassIndex(class_index))?
                    .clone();
                let result = match info.instance_format {
                    Format::VarPointers | Format::FixedAndVar | Format::Weak | Format::FixedPointers | Format::Empty | Format::Words => {
                        self.new_instance(class_index, size)?
                    }
                    Format::Bytes8 | Format::Bytes16 | Format::Bytes24 | Format::Bytes32 => {
                        self.heap.allocate_bytes_in(class_index, &vec![0u8; size], Generation::Old)
                    }
                    Format::CompiledMethod => {
                        return Err(VmError::TypeError("cannot use new: with CompiledMethod"));
                    }
                };
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_CLASS => {
                let class_index = self.class_of(receiver)?;
                let class_oop = self
                    .class_table
                    .class_oop(class_index)
                    .ok_or(VmError::InvalidClassIndex(class_index))?;
                Ok(ExecOutcome::Returned(class_oop))
            }
            PRIMITIVE_AT => {
                let result = self.primitive_at(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_AT_PUT => {
                let result = self.primitive_at_put(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_SUBCLASS => {
                let result = self.primitive_subclass(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_SUBCLASS_EXTENDED => {
                let result = self.primitive_subclass_extended(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_THIS_CONTEXT => {
                let result = self.materialize_current_context().unwrap_or_else(Oop::nil);
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_SIZE => {
                let result = self.primitive_size(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_COPY_FROM_TO => {
                let result = self.primitive_copy_from_to(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_INSTALL_METHOD => {
                let result = self.primitive_install_method(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_INSTALL_COMPILED_METHOD => {
                let result = self.primitive_install_compiled_method(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_HOST_DISPLAY_OPEN => {
                let result = self.primitive_host_display_open(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_HOST_DISPLAY_PRESENT_FORM => {
                let result = self.primitive_host_display_present_form(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_HOST_DISPLAY_SAVE_PNG => {
                let result = self.primitive_host_display_save_png(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_HOST_NEXT_EVENT => {
                let result = self.primitive_host_next_event(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_MILLISECOND_CLOCK => {
                let result = self.primitive_millisecond_clock(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_SLEEP_MILLISECONDS => {
                let result = self.primitive_sleep_milliseconds(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_FORM_FILL_RECTANGLE => {
                let result = self.primitive_form_fill_rectangle(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_FORM_COPY_RECTANGLE => {
                let result = self.primitive_form_copy_rectangle(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_GLOBAL_ASSOCIATION => {
                let result = self.primitive_global_association(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_COMPILED_METHOD => {
                let result = self.primitive_compiled_method(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_EQUALS => {
                let result = self.primitive_equals(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_INTERN_SYMBOL => {
                let result = self.primitive_intern_symbol(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            PRIMITIVE_INSTANCE_VARIABLE_INDEX => {
                let result = self.primitive_instance_variable_index(receiver, args)?;
                Ok(ExecOutcome::Returned(result))
            }
            index => Err(VmError::PrimitiveFailed(index)),
        }
    }

    fn activate_closure(&mut self, closure: Oop, args: &[Oop]) -> Result<ExecOutcome, VmError> {
        let home = self
            .closure_homes
            .get(&closure)
            .copied()
            .ok_or(VmError::TypeError("unknown closure home"))?;
        let method = self
            .heap
            .read_slot(closure, 3)
            .ok_or(VmError::TypeError("malformed BlockClosure"))?;
        let start_ip = self
            .heap
            .read_slot(closure, 1)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("malformed BlockClosure"))? as usize;
        let owner_class = self
            .owner_class_of_method(method)
            .unwrap_or(CLASS_INDEX_BLOCK_CLOSURE);
        let copied_count = self.heap.slot_count(closure).unwrap_or(4).saturating_sub(4);
        let mut all_args = Vec::with_capacity(copied_count + args.len());
        for index in 0..copied_count {
            all_args.push(
                self.heap
                    .read_slot(closure, 4 + index)
                    .ok_or(VmError::TypeError("malformed BlockClosure"))?,
            );
        }
        all_args.extend_from_slice(args);
        self.execute_method_internal(
            owner_class,
            method,
            home.receiver,
            &all_args,
            start_ip,
            Some(home.home_frame_id),
        )
    }

    fn primitive_at(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount {
                expected: 1,
                actual: args.len(),
            });
        }
        let index = args[0]
            .as_i64()
            .ok_or(VmError::TypeError("at: expects SmallInteger index"))?
            as isize;
        if index <= 0 {
            return Err(VmError::IndexOutOfBounds {
                index: index.max(0) as usize,
                size: 0,
            });
        }
        let index = (index - 1) as usize;
        match self.class_of(receiver)? {
            CLASS_INDEX_ARRAY => {
                let size = self.heap.slot_count(receiver).unwrap_or(0);
                if index >= size {
                    return Err(VmError::IndexOutOfBounds { index, size });
                }
                self.heap
                    .read_slot(receiver, index)
                    .ok_or(VmError::IndexOutOfBounds { index, size })
            }
            CLASS_INDEX_BYTE_ARRAY | CLASS_INDEX_STRING | CLASS_INDEX_SYMBOL => {
                let bytes = self
                    .heap
                    .bytes(receiver)
                    .ok_or(VmError::TypeError("expected byte object"))?;
                if index >= bytes.len() {
                    return Err(VmError::IndexOutOfBounds {
                        index,
                        size: bytes.len(),
                    });
                }
                Oop::from_i64(bytes[index] as i64).ok_or(VmError::TypeError("byte value overflow"))
            }
            _ => Err(VmError::TypeError("at: unsupported receiver")),
        }
    }

    fn primitive_at_put(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 2 {
            return Err(VmError::WrongArgumentCount {
                expected: 2,
                actual: args.len(),
            });
        }
        let index = args[0]
            .as_i64()
            .ok_or(VmError::TypeError("at:put: expects SmallInteger index"))?
            as isize;
        if index <= 0 {
            return Err(VmError::IndexOutOfBounds {
                index: index.max(0) as usize,
                size: 0,
            });
        }
        let index = (index - 1) as usize;
        match self.class_of(receiver)? {
            CLASS_INDEX_ARRAY => {
                let size = self.heap.slot_count(receiver).unwrap_or(0);
                if index >= size {
                    return Err(VmError::IndexOutOfBounds { index, size });
                }
                self.heap.write_slot(receiver, index, args[1]);
                Ok(args[1])
            }
            CLASS_INDEX_BYTE_ARRAY | CLASS_INDEX_STRING | CLASS_INDEX_SYMBOL => {
                let byte = args[1]
                    .as_i64()
                    .ok_or(VmError::TypeError("byte at:put: expects SmallInteger value"))?;
                if !(0..=255).contains(&byte) {
                    return Err(VmError::TypeError("byte at:put: expects 0..255"));
                }
                let size = self.heap.byte_len(receiver).unwrap_or(0);
                if index >= size {
                    return Err(VmError::IndexOutOfBounds { index, size });
                }
                self.heap
                    .write_byte(receiver, index, byte as u8)
                    .ok_or(VmError::TypeError("expected byte object"))?;
                Ok(args[1])
            }
            _ => Err(VmError::TypeError("at:put: unsupported receiver")),
        }
    }

    fn primitive_size(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if !args.is_empty() {
            return Err(VmError::WrongArgumentCount { expected: 0, actual: args.len() });
        }
        let size = if receiver.is_nil() || receiver.is_small_int() {
            0usize
        } else {
            match self.class_of(receiver)? {
                CLASS_INDEX_BYTE_ARRAY | CLASS_INDEX_STRING | CLASS_INDEX_SYMBOL => {
                    self.heap.bytes(receiver).map(|b| b.len()).unwrap_or(0)
                }
                _ => self.heap.slot_count(receiver).unwrap_or(0),
            }
        };
        Oop::from_i64(size as i64).ok_or(VmError::TypeError("size overflow"))
    }

    fn primitive_copy_from_to(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 2 {
            return Err(VmError::WrongArgumentCount { expected: 2, actual: args.len() });
        }
        let start = args[0]
            .as_i64()
            .ok_or(VmError::TypeError("copyFrom:to: expects SmallInteger start"))?;
        let stop = args[1]
            .as_i64()
            .ok_or(VmError::TypeError("copyFrom:to: expects SmallInteger stop"))?;
        if start <= 0 || stop < start {
            return Err(VmError::IndexOutOfBounds { index: 0, size: 0 });
        }
        let start_index = (start - 1) as usize;
        let stop_index = stop as usize;
        match self.class_of(receiver)? {
            CLASS_INDEX_BYTE_ARRAY | CLASS_INDEX_STRING | CLASS_INDEX_SYMBOL => {
                let bytes = self
                    .heap
                    .bytes(receiver)
                    .ok_or(VmError::TypeError("expected byte object"))?;
                if stop_index > bytes.len() {
                    return Err(VmError::IndexOutOfBounds { index: stop_index, size: bytes.len() });
                }
                Ok(self.heap.allocate_bytes_in(
                    self.class_of(receiver)?,
                    &bytes[start_index..stop_index],
                    Generation::Old,
                ))
            }
            CLASS_INDEX_ARRAY => {
                let size = self.heap.slot_count(receiver).unwrap_or(0);
                if stop_index > size {
                    return Err(VmError::IndexOutOfBounds { index: stop_index, size });
                }
                let mut values = Vec::with_capacity(stop_index - start_index);
                for index in start_index..stop_index {
                    values.push(self.heap.read_slot(receiver, index).ok_or(VmError::IndexOutOfBounds { index, size })?);
                }
                Ok(self.make_array(&values))
            }
            _ => Err(VmError::TypeError("copyFrom:to: unsupported receiver")),
        }
    }

    fn primitive_install_method(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 5 {
            return Err(VmError::WrongArgumentCount { expected: 5, actual: args.len() });
        }
        let class_index = self
            .class_table
            .class_index_of_oop(receiver)
            .ok_or(VmError::TypeError("installMethod... expects a class receiver"))?;
        let selector_text = self.symbol_text(args[0])?;
        let selector = self.intern_symbol(&selector_text);
        let literals_size = self.heap.slot_count(args[1]).unwrap_or(0);
        let mut literals = Vec::with_capacity(literals_size);
        for index in 0..literals_size {
            literals.push(self.heap.read_slot(args[1], index).ok_or(VmError::IndexOutOfBounds { index, size: literals_size })?);
        }
        let bytecodes_size = self.heap.slot_count(args[2]).unwrap_or(0);
        let mut bytecodes = Vec::with_capacity(bytecodes_size);
        for index in 0..bytecodes_size {
            let value = self
                .heap
                .read_slot(args[2], index)
                .and_then(Oop::as_i64)
                .ok_or(VmError::TypeError("bytecode array must contain SmallIntegers"))?;
            bytecodes.push(value as u8);
        }
        let num_args = args[3]
            .as_i64()
            .ok_or(VmError::TypeError("numArgs must be SmallInteger"))? as u8;
        let num_temps = args[4]
            .as_i64()
            .ok_or(VmError::TypeError("numTemps must be SmallInteger"))? as u8;
        let method = self.compiled_method(
            MethodHeaderFields {
                num_args,
                num_temps,
                num_literals: literals.len() as u16,
                flags: 0,
            },
            &literals,
            &bytecodes,
        );
        self.add_method(class_index, selector, method)?;
        Ok(method)
    }

    fn primitive_install_compiled_method(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 2 {
            return Err(VmError::WrongArgumentCount { expected: 2, actual: args.len() });
        }
        let class_index = self
            .class_table
            .class_index_of_oop(receiver)
            .ok_or(VmError::TypeError("installCompiledMethod:selector: expects a class receiver"))?;
        let selector_text = self.symbol_text(args[1])?;
        let selector = self.intern_symbol(&selector_text);
        let method = args[0];
        if self.heap.compiled_method_header(method).is_none() {
            return Err(VmError::TypeError("installCompiledMethod:selector: expects a CompiledMethod"));
        }
        self.add_method(class_index, selector, method)?;
        Ok(method)
    }

    fn primitive_host_display_open(&mut self, _receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 3 {
            return Err(VmError::WrongArgumentCount { expected: 3, actual: args.len() });
        }
        let width = args[0]
            .as_i64()
            .ok_or(VmError::TypeError("width must be SmallInteger"))? as usize;
        let height = args[1]
            .as_i64()
            .ok_or(VmError::TypeError("height must be SmallInteger"))? as usize;
        let depth = args[2]
            .as_i64()
            .ok_or(VmError::TypeError("depth must be SmallInteger"))? as usize;
        let class_index = self
            .class_index_by_name("HostDisplay")
            .ok_or(VmError::TypeError("HostDisplay class not loaded"))?;
        let object = self.new_instance(class_index, 0)?;
        let handle = self.next_host_display_id;
        self.next_host_display_id += 1;
        self.host_displays.insert(
            handle,
            HostDisplayState {
                width,
                height,
                depth,
                presents: 0,
                last_frame: Vec::new(),
            },
        );
        self.heap.write_slot(object, 0, Oop::from_i64(handle as i64).unwrap());
        self.heap.write_slot(object, 1, Oop::from_i64(width as i64).unwrap());
        self.heap.write_slot(object, 2, Oop::from_i64(height as i64).unwrap());
        self.heap.write_slot(object, 3, Oop::from_i64(depth as i64).unwrap());
        Ok(object)
    }

    fn primitive_host_display_present_form(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
        }
        let handle = self
            .heap
            .read_slot(receiver, 0)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("HostDisplay handle missing"))? as u32;
        let form = args[0];
        let form_class = self.class_of(form)?;
        let form_info = self
            .class_table
            .get(form_class)
            .ok_or(VmError::InvalidClassIndex(form_class))?;
        if form_info.name != "Form" {
            return Err(VmError::TypeError("presentForm: expects a Form"));
        }
        let width = self
            .heap
            .read_slot(form, 0)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("Form width missing"))? as usize;
        let height = self
            .heap
            .read_slot(form, 1)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("Form height missing"))? as usize;
        let depth = self
            .heap
            .read_slot(form, 2)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("Form depth missing"))? as usize;
        let bits = self
            .heap
            .read_slot(form, 3)
            .ok_or(VmError::TypeError("Form bits missing"))?;
        let bytes = self
            .heap
            .bytes(bits)
            .ok_or(VmError::TypeError("Form bits must be a ByteArray"))?;
        let expected = match depth {
            1 => (width * height).div_ceil(8),
            8 => width * height,
            _ => bytes.len(),
        };
        if bytes.len() < expected {
            return Err(VmError::IndexOutOfBounds { index: bytes.len(), size: expected });
        }
        let state = self
            .host_displays
            .get_mut(&handle)
            .ok_or(VmError::TypeError("unknown HostDisplay handle"))?;
        state.width = width;
        state.height = height;
        state.depth = depth;
        state.last_frame = bytes;
        state.presents += 1;
        Ok(receiver)
    }

    fn primitive_host_display_save_png(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
        }
        let handle = self
            .heap
            .read_slot(receiver, 0)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("HostDisplay handle missing"))? as u32;
        let path = self.symbol_text(args[0])?;
        let state = self
            .host_displays
            .get(&handle)
            .ok_or(VmError::TypeError("unknown HostDisplay handle"))?;
        gui_snapshot::write_display_png(
            &path,
            state.width,
            state.height,
            state.depth,
            &state.last_frame,
        )
        .map_err(|_| VmError::PrimitiveFailed(PRIMITIVE_HOST_DISPLAY_SAVE_PNG))?;
        Ok(receiver)
    }

    fn host_event_to_oop(&mut self, event: HostEvent) -> Oop {
        let values: Vec<Oop> = match event {
            HostEvent::MouseMove { x, y } => vec![self.intern_symbol("mouseMove"), Oop::from_i64(x).unwrap(), Oop::from_i64(y).unwrap()],
            HostEvent::MouseDown { x, y, button } => vec![
                self.intern_symbol("mouseDown"),
                Oop::from_i64(x).unwrap(),
                Oop::from_i64(y).unwrap(),
                Oop::from_i64(button).unwrap(),
            ],
            HostEvent::MouseUp { x, y, button } => vec![
                self.intern_symbol("mouseUp"),
                Oop::from_i64(x).unwrap(),
                Oop::from_i64(y).unwrap(),
                Oop::from_i64(button).unwrap(),
            ],
            HostEvent::KeyDown { key } => vec![self.intern_symbol("keyDown"), Oop::from_i64(key).unwrap()],
            HostEvent::KeyUp { key } => vec![self.intern_symbol("keyUp"), Oop::from_i64(key).unwrap()],
            HostEvent::Resize { width, height } => vec![
                self.intern_symbol("resize"),
                Oop::from_i64(width).unwrap(),
                Oop::from_i64(height).unwrap(),
            ],
            HostEvent::Quit => vec![self.intern_symbol("quit")],
        };
        self.make_array(&values)
    }

    fn primitive_host_next_event(&mut self, _receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if !args.is_empty() {
            return Err(VmError::WrongArgumentCount { expected: 0, actual: args.len() });
        }
        Ok(self
            .host_events
            .pop_front()
            .map(|event| self.host_event_to_oop(event))
            .unwrap_or_else(Oop::nil))
    }

    fn primitive_millisecond_clock(&mut self, _receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if !args.is_empty() {
            return Err(VmError::WrongArgumentCount { expected: 0, actual: args.len() });
        }
        let millis = self.clock_start.elapsed().as_millis() as i64;
        Oop::from_i64(millis).ok_or(VmError::TypeError("millisecond clock overflow"))
    }

    fn primitive_sleep_milliseconds(&mut self, _receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
        }
        let millis = args[0]
            .as_i64()
            .ok_or(VmError::TypeError("sleepMilliseconds: expects SmallInteger"))?;
        if millis > 0 {
            std::thread::sleep(Duration::from_millis(millis as u64));
        }
        Ok(Oop::nil())
    }

    fn primitive_form_fill_rectangle(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 5 {
            return Err(VmError::WrongArgumentCount { expected: 5, actual: args.len() });
        }
        let x = args[0]
            .as_i64()
            .ok_or(VmError::TypeError("x must be SmallInteger"))?
            .max(0) as usize;
        let y = args[1]
            .as_i64()
            .ok_or(VmError::TypeError("y must be SmallInteger"))?
            .max(0) as usize;
        let width = args[2]
            .as_i64()
            .ok_or(VmError::TypeError("width must be SmallInteger"))?
            .max(0) as usize;
        let height = args[3]
            .as_i64()
            .ok_or(VmError::TypeError("height must be SmallInteger"))?
            .max(0) as usize;
        let value = args[4]
            .as_i64()
            .ok_or(VmError::TypeError("fill value must be SmallInteger"))?;
        let form_class = self.class_of(receiver)?;
        let form_info = self
            .class_table
            .get(form_class)
            .ok_or(VmError::InvalidClassIndex(form_class))?;
        if form_info.name != "Form" {
            return Err(VmError::TypeError("fillRectangle... expects a Form receiver"));
        }
        let form_width = self
            .heap
            .read_slot(receiver, 0)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("Form width missing"))? as usize;
        let form_height = self
            .heap
            .read_slot(receiver, 1)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("Form height missing"))? as usize;
        let depth = self
            .heap
            .read_slot(receiver, 2)
            .and_then(Oop::as_i64)
            .ok_or(VmError::TypeError("Form depth missing"))? as usize;
        let bits = self
            .heap
            .read_slot(receiver, 3)
            .ok_or(VmError::TypeError("Form bits missing"))?;
        match depth {
            1 => {
                let mut bytes = self
                    .heap
                    .bytes(bits)
                    .ok_or(VmError::TypeError("Form bits must be a ByteArray"))?;
                let max_y = y.saturating_add(height).min(form_height);
                let max_x = x.saturating_add(width).min(form_width);
                for py in y..max_y {
                    for px in x..max_x {
                        let pixel_index = py * form_width + px;
                        let byte_index = pixel_index / 8;
                        let bit_index = 7 - (pixel_index % 8);
                        let mask = 1u8 << bit_index;
                        if value == 0 {
                            bytes[byte_index] &= !mask;
                        } else {
                            bytes[byte_index] |= mask;
                        }
                    }
                }
                for (index, byte) in bytes.into_iter().enumerate() {
                    let _ = self.heap.write_byte(bits, index, byte);
                }
            }
            8 => {
                let mut bytes = self
                    .heap
                    .bytes(bits)
                    .ok_or(VmError::TypeError("Form bits must be a ByteArray"))?;
                let fill = value.clamp(0, 255) as u8;
                let max_y = y.saturating_add(height).min(form_height);
                let max_x = x.saturating_add(width).min(form_width);
                for py in y..max_y {
                    let row_start = py * form_width;
                    for px in x..max_x {
                        bytes[row_start + px] = fill;
                    }
                }
                for (index, byte) in bytes.into_iter().enumerate() {
                    let _ = self.heap.write_byte(bits, index, byte);
                }
            }
            _ => return Err(VmError::TypeError("fillRectangle... only supports depth 1 and 8 forms")),
        }
        Ok(receiver)
    }

    fn primitive_form_copy_rectangle(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 7 {
            return Err(VmError::WrongArgumentCount { expected: 7, actual: args.len() });
        }
        let dest_x = args[0]
            .as_i64()
            .ok_or(VmError::TypeError("dest x must be SmallInteger"))?
            .max(0) as usize;
        let dest_y = args[1]
            .as_i64()
            .ok_or(VmError::TypeError("dest y must be SmallInteger"))?
            .max(0) as usize;
        let width = args[2]
            .as_i64()
            .ok_or(VmError::TypeError("width must be SmallInteger"))?
            .max(0) as usize;
        let height = args[3]
            .as_i64()
            .ok_or(VmError::TypeError("height must be SmallInteger"))?
            .max(0) as usize;
        let source_form = args[4];
        let source_x = args[5]
            .as_i64()
            .ok_or(VmError::TypeError("source x must be SmallInteger"))?
            .max(0) as usize;
        let source_y = args[6]
            .as_i64()
            .ok_or(VmError::TypeError("source y must be SmallInteger"))?
            .max(0) as usize;

        let dest_class = self.class_of(receiver)?;
        let dest_info = self
            .class_table
            .get(dest_class)
            .ok_or(VmError::InvalidClassIndex(dest_class))?;
        if dest_info.name != "Form" {
            return Err(VmError::TypeError("copyRectangle... expects a Form receiver"));
        }
        let source_class = self.class_of(source_form)?;
        let source_info = self
            .class_table
            .get(source_class)
            .ok_or(VmError::InvalidClassIndex(source_class))?;
        if source_info.name != "Form" {
            return Err(VmError::TypeError("copyRectangle... expects a Form source"));
        }

        let dest_width = self.heap.read_slot(receiver, 0).and_then(Oop::as_i64).ok_or(VmError::TypeError("Form width missing"))? as usize;
        let dest_height = self.heap.read_slot(receiver, 1).and_then(Oop::as_i64).ok_or(VmError::TypeError("Form height missing"))? as usize;
        let dest_depth = self.heap.read_slot(receiver, 2).and_then(Oop::as_i64).ok_or(VmError::TypeError("Form depth missing"))? as usize;
        let dest_bits = self.heap.read_slot(receiver, 3).ok_or(VmError::TypeError("Form bits missing"))?;

        let source_width = self.heap.read_slot(source_form, 0).and_then(Oop::as_i64).ok_or(VmError::TypeError("Form width missing"))? as usize;
        let source_height = self.heap.read_slot(source_form, 1).and_then(Oop::as_i64).ok_or(VmError::TypeError("Form height missing"))? as usize;
        let source_depth = self.heap.read_slot(source_form, 2).and_then(Oop::as_i64).ok_or(VmError::TypeError("Form depth missing"))? as usize;
        let source_bits = self.heap.read_slot(source_form, 3).ok_or(VmError::TypeError("Form bits missing"))?;

        if dest_depth != source_depth {
            return Err(VmError::TypeError("copyRectangle... requires matching form depths"));
        }

        match dest_depth {
            1 => {
                let source_bytes = self.heap.bytes(source_bits).ok_or(VmError::TypeError("Form bits must be a ByteArray"))?;
                let mut dest_bytes = self.heap.bytes(dest_bits).ok_or(VmError::TypeError("Form bits must be a ByteArray"))?;
                let max_y = height
                    .min(dest_height.saturating_sub(dest_y))
                    .min(source_height.saturating_sub(source_y));
                let max_x = width
                    .min(dest_width.saturating_sub(dest_x))
                    .min(source_width.saturating_sub(source_x));
                for row in 0..max_y {
                    for col in 0..max_x {
                        let src_pixel = (source_y + row) * source_width + (source_x + col);
                        let src_byte = source_bytes[src_pixel / 8];
                        let src_mask = 1u8 << (7 - (src_pixel % 8));
                        let bit_on = (src_byte & src_mask) != 0;
                        let dst_pixel = (dest_y + row) * dest_width + (dest_x + col);
                        let dst_byte_index = dst_pixel / 8;
                        let dst_mask = 1u8 << (7 - (dst_pixel % 8));
                        if bit_on {
                            dest_bytes[dst_byte_index] |= dst_mask;
                        } else {
                            dest_bytes[dst_byte_index] &= !dst_mask;
                        }
                    }
                }
                for (index, byte) in dest_bytes.into_iter().enumerate() {
                    let _ = self.heap.write_byte(dest_bits, index, byte);
                }
            }
            8 => {
                let source_bytes = self.heap.bytes(source_bits).ok_or(VmError::TypeError("Form bits must be a ByteArray"))?;
                let mut dest_bytes = self.heap.bytes(dest_bits).ok_or(VmError::TypeError("Form bits must be a ByteArray"))?;
                let max_y = height
                    .min(dest_height.saturating_sub(dest_y))
                    .min(source_height.saturating_sub(source_y));
                let max_x = width
                    .min(dest_width.saturating_sub(dest_x))
                    .min(source_width.saturating_sub(source_x));
                for row in 0..max_y {
                    for col in 0..max_x {
                        let src_index = (source_y + row) * source_width + (source_x + col);
                        let dst_index = (dest_y + row) * dest_width + (dest_x + col);
                        dest_bytes[dst_index] = source_bytes[src_index];
                    }
                }
                for (index, byte) in dest_bytes.into_iter().enumerate() {
                    let _ = self.heap.write_byte(dest_bits, index, byte);
                }
            }
            _ => return Err(VmError::TypeError("copyRectangle... only supports depth 1 and 8 forms")),
        }
        Ok(receiver)
    }

    fn primitive_global_association(&mut self, _receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
        }
        let name = self.symbol_text(args[0])?;
        Ok(self.global_association(&name))
    }

    fn primitive_compiled_method(&mut self, _receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 5 {
            return Err(VmError::WrongArgumentCount { expected: 5, actual: args.len() });
        }
        let literals_size = self.heap.slot_count(args[1]).unwrap_or(0);
        let mut literals = Vec::with_capacity(literals_size);
        for index in 0..literals_size {
            literals.push(self.heap.read_slot(args[1], index).ok_or(VmError::IndexOutOfBounds { index, size: literals_size })?);
        }
        let bytecodes_size = self.heap.slot_count(args[2]).unwrap_or(0);
        let mut bytecodes = Vec::with_capacity(bytecodes_size);
        for index in 0..bytecodes_size {
            let value = self
                .heap
                .read_slot(args[2], index)
                .and_then(Oop::as_i64)
                .ok_or(VmError::TypeError("bytecode array must contain SmallIntegers"))?;
            bytecodes.push(value as u8);
        }
        let num_args = args[3]
            .as_i64()
            .ok_or(VmError::TypeError("numArgs must be SmallInteger"))? as u8;
        let num_temps = args[4]
            .as_i64()
            .ok_or(VmError::TypeError("numTemps must be SmallInteger"))? as u8;
        Ok(self.compiled_method(
            MethodHeaderFields {
                num_args,
                num_temps,
                num_literals: literals.len() as u16,
                flags: 0,
            },
            &literals,
            &bytecodes,
        ))
    }

    fn primitive_equals(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
        }
        Ok(self.boolean_oop(receiver == args[0]))
    }

    fn primitive_intern_symbol(&mut self, _receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
        }
        let text = self.symbol_text(args[0])?;
        Ok(self.intern_symbol(&text))
    }

    fn primitive_instance_variable_index(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 1 {
            return Err(VmError::WrongArgumentCount { expected: 1, actual: args.len() });
        }
        let class_index = self
            .class_table
            .class_index_of_oop(receiver)
            .ok_or(VmError::TypeError("instanceVariableIndex: expects a class receiver"))?;
        let name = self.symbol_text(args[0])?;
        match self.class_table.instance_variable_index(class_index, &name) {
            Some(index) => Oop::from_i64(index as i64).ok_or(VmError::TypeError("ivar index overflow")),
            None => Ok(Oop::nil()),
        }
    }

    fn primitive_subclass(&mut self, receiver: Oop, args: &[Oop]) -> Result<Oop, VmError> {
        if args.len() != 2 {
            return Err(VmError::WrongArgumentCount {
                expected: 2,
                actual: args.len(),
            });
        }
        self.primitive_subclass_from_parts(receiver, args[0], args[1])
    }

    fn primitive_subclass_extended(
        &mut self,
        receiver: Oop,
        args: &[Oop],
    ) -> Result<Oop, VmError> {
        if args.len() != 5 {
            return Err(VmError::WrongArgumentCount {
                expected: 5,
                actual: args.len(),
            });
        }
        self.primitive_subclass_from_parts(receiver, args[0], args[1])
    }

    fn primitive_subclass_from_parts(
        &mut self,
        receiver: Oop,
        class_name_oop: Oop,
        ivar_names_oop: Oop,
    ) -> Result<Oop, VmError> {
        let superclass_index = self
            .class_table
            .class_index_of_oop(receiver)
            .ok_or(VmError::TypeError("subclass primitive expects a class receiver"))?;
        let class_name = self.symbol_text(class_name_oop)?;
        let ivar_text = if ivar_names_oop.is_nil() {
            String::new()
        } else {
            self.symbol_text(ivar_names_oop)?
        };
        let ivars = ivar_text
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let inherited_fixed_fields = if superclass_index == CLASS_INDEX_BEHAVIOR {
            0
        } else {
            self.class_table
                .get(superclass_index)
                .ok_or(VmError::InvalidClassIndex(superclass_index))?
                .fixed_fields
        };
        let expected_fixed_fields = inherited_fixed_fields + ivars.len();
        let class_index = if let Some(existing) = self
            .class_table
            .iter()
            .find_map(|(index, info)| (info.name == class_name).then_some(index))
        {
            let info = self
                .class_table
                .get(existing)
                .ok_or(VmError::InvalidClassIndex(existing))?;
            if info.superclass != Some(superclass_index)
                || info.fixed_fields != expected_fixed_fields
                || info.instance_format != Format::FixedPointers
            {
                return Err(VmError::TypeError("existing class shape mismatch"));
            }
            existing
        } else {
            self.new_class(
                &class_name,
                Some(superclass_index),
                Format::FixedPointers,
                ivars.len(),
            )?
        };
        self.set_instance_variables(class_index, ivars)?;
        let class_oop = self
            .class_table
            .class_oop(class_index)
            .ok_or(VmError::InvalidClassIndex(class_index))?;
        self.set_global(&class_name, class_oop);
        Ok(class_oop)
    }

    fn temp_slot_index(_argc: usize, index: usize) -> usize {
        4 + index
    }

    fn push_frame_header(
        &mut self,
        previous_fp: usize,
        start_ip: usize,
        method: Oop,
        receiver: Oop,
    ) {
        self.stack.push(Oop::from_i64(previous_fp as i64).unwrap());
        self.stack.push(Oop::from_i64(start_ip as i64).unwrap());
        self.stack.push(method);
        self.stack.push(receiver);
    }

    fn execute_method_internal(
        &mut self,
        owner_class: u32,
        method: Oop,
        receiver: Oop,
        args: &[Oop],
        start_ip: usize,
        block_home: Option<usize>,
    ) -> Result<ExecOutcome, VmError> {
        let header = self
            .heap
            .compiled_method_header(method)
            .ok_or(VmError::InvalidMethod(method))?;
        if args.len() != header.num_args as usize {
            return Err(VmError::WrongArgumentCount {
                expected: header.num_args as usize,
                actual: args.len(),
            });
        }
        let bytecodes = self
            .heap
            .compiled_method_bytecodes(method)
            .ok_or(VmError::InvalidMethod(method))?;

        let previous_fp = self.stack.fp;
        let frame_base = self.stack.sp;
        self.stack.fp = frame_base;
        self.push_frame_header(previous_fp, start_ip, method, receiver);
        for arg in args.iter().copied() {
            self.stack.push(arg);
        }
        for _ in 0..header.num_temps {
            self.stack.push(Oop::nil());
        }

        let frame_id = self.next_frame_id;
        self.next_frame_id += 1;
        self.active_frame_ids.push(frame_id);

        macro_rules! exit_frame {
            ($outcome:expr) => {{
                self.active_frame_ids.pop();
                self.stack.truncate(frame_base);
                self.stack.fp = previous_fp;
                return Ok($outcome);
            }};
        }

        let mut ip = start_ip;
        while ip < bytecodes.len() {
            let opcode = bytecodes[ip];
            ip += 1;
            match opcode {
                0x00..=0x0f => {
                    let index = (opcode - PUSH_INST_VAR_BASE) as usize;
                    self.stack.push(self.heap.read_slot(receiver, index).ok_or(
                        VmError::IndexOutOfBounds {
                            index,
                            size: self.heap.slot_count(receiver).unwrap_or(0),
                        },
                    )?);
                }
                0x10..=0x1f => {
                    let index = (opcode - PUSH_TEMP_BASE) as usize;
                    self.stack.push(
                        self.stack
                            .get(frame_base + Self::temp_slot_index(args.len(), index))?,
                    );
                }
                0x20..=0x2f => {
                    let index = (opcode - PUSH_LITERAL_BASE) as usize;
                    self.stack
                        .push(self.heap.compiled_method_literal(method, index).ok_or(
                            VmError::IndexOutOfBounds {
                                index,
                                size: header.num_literals as usize,
                            },
                        )?);
                }
                0x30..=0x3f => {
                    let index = (opcode - PUSH_LIT_VAR_BASE) as usize;
                    let assoc = self.heap.compiled_method_literal(method, index).ok_or(
                        VmError::IndexOutOfBounds {
                            index,
                            size: header.num_literals as usize,
                        },
                    )?;
                    self.stack.push(
                        self.heap
                            .read_slot(assoc, 1)
                            .ok_or(VmError::TypeError("expected Association"))?,
                    );
                }
                0x40..=0x47 => {
                    let index = (opcode - POP_STORE_INST_VAR_BASE) as usize;
                    let value = self.stack.pop()?;
                    self.heap.write_slot(receiver, index, value);
                }
                0x48..=0x4f => {
                    let index = (opcode - POP_STORE_TEMP_BASE) as usize;
                    let value = self.stack.pop()?;
                    self.stack
                        .set(frame_base + Self::temp_slot_index(args.len(), index), value)?;
                }
                PUSH_SELF => self.stack.push(receiver),
                PUSH_NIL => self.stack.push(Oop::nil()),
                PUSH_TRUE => self.stack.push(self.true_oop()),
                PUSH_FALSE => self.stack.push(self.false_oop()),
                PUSH_MINUS_ONE => self.stack.push(Oop::from_i64(-1).unwrap()),
                PUSH_ZERO => self.stack.push(Oop::from_i64(0).unwrap()),
                PUSH_ONE => self.stack.push(Oop::from_i64(1).unwrap()),
                PUSH_TWO => self.stack.push(Oop::from_i64(2).unwrap()),
                DUP => self.stack.push(self.stack.peek()?),
                POP => {
                    let _ = self.stack.pop()?;
                }
                0x60..=0x6f => {
                    let literal_index = (opcode - SEND_SHORT_BASE) as usize;
                    let selector = self
                        .heap
                        .compiled_method_literal(method, literal_index)
                        .ok_or(VmError::IndexOutOfBounds {
                            index: literal_index,
                            size: header.num_literals as usize,
                        })?;
                    let argc = selector_arity(&self.symbol_text(selector)?);
                    let mut send_args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        send_args.push(self.stack.pop()?);
                    }
                    send_args.reverse();
                    let rcvr = self.stack.pop()?;
                    match self.send_message(rcvr, selector, &send_args, None)? {
                        ExecOutcome::Returned(value) => self.stack.push(value),
                        ExecOutcome::NonLocalReturn {
                            home_frame_id,
                            result,
                        } => {
                            if home_frame_id == frame_id {
                                exit_frame!(ExecOutcome::Returned(result));
                            }
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result
                            });
                        }
                    }
                }
                0x70..=0x7e => {
                    let special_index = (opcode - SEND_SPECIAL_BASE) as usize;
                    let arg = self.stack.pop()?;
                    let rcvr = self.stack.pop()?;
                    if let Some(result) = self.try_fast_special_send(special_index, rcvr, arg)? {
                        self.stack.push(result);
                    } else {
                        let selector = self.special_selectors[special_index];
                        match self.send_message(rcvr, selector, &[arg], None)? {
                            ExecOutcome::Returned(value) => self.stack.push(value),
                            ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result,
                            } => {
                                if home_frame_id == frame_id {
                                    exit_frame!(ExecOutcome::Returned(result));
                                }
                                exit_frame!(ExecOutcome::NonLocalReturn {
                                    home_frame_id,
                                    result
                                });
                            }
                        }
                    }
                }
                0x7f => {
                    let value = self.stack.pop()?;
                    let index_oop = self.stack.pop()?;
                    let rcvr = self.stack.pop()?;
                    if let Some(result) = self.try_fast_at_put(rcvr, index_oop, value)? {
                        self.stack.push(result);
                    } else {
                        let selector = self.special_selectors[15];
                        match self.send_message(rcvr, selector, &[index_oop, value], None)? {
                            ExecOutcome::Returned(value) => self.stack.push(value),
                            ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result,
                            } => {
                                if home_frame_id == frame_id {
                                    exit_frame!(ExecOutcome::Returned(result));
                                }
                                exit_frame!(ExecOutcome::NonLocalReturn {
                                    home_frame_id,
                                    result
                                });
                            }
                        }
                    }
                }
                PUSH_INST_VAR_EXT => {
                    let index = bytecodes[ip] as usize;
                    ip += 1;
                    self.stack.push(self.heap.read_slot(receiver, index).ok_or(
                        VmError::IndexOutOfBounds {
                            index,
                            size: self.heap.slot_count(receiver).unwrap_or(0),
                        },
                    )?);
                }
                PUSH_TEMP_EXT => {
                    let index = bytecodes[ip] as usize;
                    ip += 1;
                    self.stack.push(
                        self.stack
                            .get(frame_base + Self::temp_slot_index(args.len(), index))?,
                    );
                }
                PUSH_LITERAL_EXT => {
                    let index = bytecodes[ip] as usize;
                    ip += 1;
                    self.stack
                        .push(self.heap.compiled_method_literal(method, index).ok_or(
                            VmError::IndexOutOfBounds {
                                index,
                                size: header.num_literals as usize,
                            },
                        )?);
                }
                PUSH_LIT_VAR_EXT => {
                    let index = bytecodes[ip] as usize;
                    ip += 1;
                    let assoc = self.heap.compiled_method_literal(method, index).ok_or(
                        VmError::IndexOutOfBounds {
                            index,
                            size: header.num_literals as usize,
                        },
                    )?;
                    self.stack.push(
                        self.heap
                            .read_slot(assoc, 1)
                            .ok_or(VmError::TypeError("expected Association"))?,
                    );
                }
                POP_STORE_INST_VAR_EXT => {
                    let index = bytecodes[ip] as usize;
                    ip += 1;
                    let value = self.stack.pop()?;
                    self.heap.write_slot(receiver, index, value);
                }
                POP_STORE_TEMP_EXT => {
                    let index = bytecodes[ip] as usize;
                    ip += 1;
                    let value = self.stack.pop()?;
                    self.stack
                        .set(frame_base + Self::temp_slot_index(args.len(), index), value)?;
                }
                SEND_EXT => {
                    let literal_index = bytecodes[ip] as usize;
                    ip += 1;
                    let selector = self
                        .heap
                        .compiled_method_literal(method, literal_index)
                        .ok_or(VmError::IndexOutOfBounds {
                            index: literal_index,
                            size: header.num_literals as usize,
                        })?;
                    let argc = selector_arity(&self.symbol_text(selector)?);
                    let mut send_args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        send_args.push(self.stack.pop()?);
                    }
                    send_args.reverse();
                    let rcvr = self.stack.pop()?;
                    match self.send_message(rcvr, selector, &send_args, None)? {
                        ExecOutcome::Returned(value) => self.stack.push(value),
                        ExecOutcome::NonLocalReturn {
                            home_frame_id,
                            result,
                        } => {
                            if home_frame_id == frame_id {
                                exit_frame!(ExecOutcome::Returned(result));
                            }
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result
                            });
                        }
                    }
                }
                SUPER_SEND_EXT => {
                    let literal_index = bytecodes[ip] as usize;
                    ip += 1;
                    let selector = self
                        .heap
                        .compiled_method_literal(method, literal_index)
                        .ok_or(VmError::IndexOutOfBounds {
                            index: literal_index,
                            size: header.num_literals as usize,
                        })?;
                    let argc = selector_arity(&self.symbol_text(selector)?);
                    let mut send_args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        send_args.push(self.stack.pop()?);
                    }
                    send_args.reverse();
                    let rcvr = self.stack.pop()?;
                    let super_start = self
                        .class_table
                        .superclass_of(owner_class)
                        .ok_or(VmError::InvalidClassIndex(owner_class))?;
                    match self.send_message(rcvr, selector, &send_args, Some(super_start))? {
                        ExecOutcome::Returned(value) => self.stack.push(value),
                        ExecOutcome::NonLocalReturn {
                            home_frame_id,
                            result,
                        } => {
                            if home_frame_id == frame_id {
                                exit_frame!(ExecOutcome::Returned(result));
                            }
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result
                            });
                        }
                    }
                }
                JUMP_FORWARD => {
                    let offset = bytecodes[ip] as usize;
                    ip += 1;
                    ip += offset;
                }
                JUMP_BACK => {
                    let offset = bytecodes[ip] as usize;
                    ip += 1;
                    ip -= offset;
                }
                JUMP_TRUE => {
                    let offset = bytecodes[ip] as usize;
                    ip += 1;
                    let cond = self.stack.pop()?;
                    if cond == self.true_oop() {
                        ip += offset;
                    }
                }
                JUMP_FALSE => {
                    let offset = bytecodes[ip] as usize;
                    ip += 1;
                    let cond = self.stack.pop()?;
                    if cond == self.false_oop() {
                        ip += offset;
                    }
                }
                PUSH_NEW_ARRAY => {
                    let count = bytecodes[ip] as usize;
                    ip += 1;
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(self.stack.pop()?);
                    }
                    values.reverse();
                    let array = self.make_array(&values);
                    self.stack.push(array);
                }
                PUSH_SMALL_INT_EXT => {
                    let value = bytecodes[ip] as i64;
                    ip += 1;
                    self.stack.push(Oop::from_i64(value).unwrap());
                }
                EXTENDED_SEND => {
                    let literal_index = bytecodes[ip] as usize;
                    let argc = bytecodes[ip + 1] as usize;
                    ip += 2;
                    let selector = self
                        .heap
                        .compiled_method_literal(method, literal_index)
                        .ok_or(VmError::IndexOutOfBounds {
                            index: literal_index,
                            size: header.num_literals as usize,
                        })?;
                    let mut send_args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        send_args.push(self.stack.pop()?);
                    }
                    send_args.reverse();
                    let rcvr = self.stack.pop()?;
                    match self.send_message(rcvr, selector, &send_args, None)? {
                        ExecOutcome::Returned(value) => self.stack.push(value),
                        ExecOutcome::NonLocalReturn {
                            home_frame_id,
                            result,
                        } => {
                            if home_frame_id == frame_id {
                                exit_frame!(ExecOutcome::Returned(result));
                            }
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result
                            });
                        }
                    }
                }
                EXTENDED_SUPER_SEND => {
                    let literal_index = bytecodes[ip] as usize;
                    let argc = bytecodes[ip + 1] as usize;
                    ip += 2;
                    let selector = self
                        .heap
                        .compiled_method_literal(method, literal_index)
                        .ok_or(VmError::IndexOutOfBounds {
                            index: literal_index,
                            size: header.num_literals as usize,
                        })?;
                    let mut send_args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        send_args.push(self.stack.pop()?);
                    }
                    send_args.reverse();
                    let rcvr = self.stack.pop()?;
                    let super_start = self
                        .class_table
                        .superclass_of(owner_class)
                        .ok_or(VmError::InvalidClassIndex(owner_class))?;
                    match self.send_message(rcvr, selector, &send_args, Some(super_start))? {
                        ExecOutcome::Returned(value) => self.stack.push(value),
                        ExecOutcome::NonLocalReturn {
                            home_frame_id,
                            result,
                        } => {
                            if home_frame_id == frame_id {
                                exit_frame!(ExecOutcome::Returned(result));
                            }
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result
                            });
                        }
                    }
                }
                JUMP_FORWARD_LONG => {
                    let offset = ((bytecodes[ip] as usize) << 8) | bytecodes[ip + 1] as usize;
                    ip += 2;
                    ip += offset;
                }
                JUMP_BACK_LONG => {
                    let offset = ((bytecodes[ip] as usize) << 8) | bytecodes[ip + 1] as usize;
                    ip += 2;
                    ip -= offset;
                }
                JUMP_TRUE_LONG => {
                    let offset = ((bytecodes[ip] as usize) << 8) | bytecodes[ip + 1] as usize;
                    ip += 2;
                    let cond = self.stack.pop()?;
                    if cond == self.true_oop() {
                        ip += offset;
                    }
                }
                JUMP_FALSE_LONG => {
                    let offset = ((bytecodes[ip] as usize) << 8) | bytecodes[ip + 1] as usize;
                    ip += 2;
                    let cond = self.stack.pop()?;
                    if cond == self.false_oop() {
                        ip += offset;
                    }
                }
                PUSH_CLOSURE => {
                    let num_args = bytecodes[ip] as usize;
                    let copied = bytecodes[ip + 1] as usize;
                    let block_size =
                        ((bytecodes[ip + 2] as usize) << 8) | bytecodes[ip + 3] as usize;
                    ip += 4;
                    let (block_method, start_of_block) = if block_size == 0 {
                        let literal_index = bytecodes[ip] as usize;
                        ip += 1;
                        (
                            self.heap
                                .compiled_method_literal(method, literal_index)
                                .ok_or(VmError::IndexOutOfBounds {
                                    index: literal_index,
                                    size: header.num_literals as usize,
                                })?,
                            0usize,
                        )
                    } else {
                        let start_of_block = ip;
                        ip += block_size;
                        (method, start_of_block)
                    };
                    let mut copied_values = Vec::with_capacity(copied);
                    for _ in 0..copied {
                        copied_values.push(self.stack.pop()?);
                    }
                    copied_values.reverse();
                    let closure = self.heap.allocate_object(
                        CLASS_INDEX_BLOCK_CLOSURE,
                        Format::FixedPointers,
                        4 + copied_values.len(),
                    );
                    let outer_context = self.materialize_current_context().unwrap_or_else(Oop::nil);
                    self.heap.write_slot(closure, 0, outer_context);
                    self.heap
                        .write_slot(closure, 1, Oop::from_i64(start_of_block as i64).unwrap());
                    self.heap
                        .write_slot(closure, 2, Oop::from_i64(num_args as i64).unwrap());
                    self.heap.write_slot(closure, 3, block_method);
                    for (index, value) in copied_values.iter().copied().enumerate() {
                        self.heap.write_slot(closure, 4 + index, value);
                    }
                    self.closure_homes.insert(
                        closure,
                        ClosureHome {
                            home_frame_id: frame_id,
                            receiver,
                        },
                    );
                    self.stack.push(closure);
                }
                RETURN_TOP => {
                    let result = self.stack.peek()?;
                    if let Some(home_frame_id) = block_home {
                        if self.active_frame_ids.contains(&home_frame_id) {
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result
                            });
                        }
                        return Err(VmError::CannotReturn);
                    }
                    exit_frame!(ExecOutcome::Returned(result));
                }
                RETURN_SELF => {
                    if let Some(home_frame_id) = block_home {
                        if self.active_frame_ids.contains(&home_frame_id) {
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result: receiver,
                            });
                        }
                        return Err(VmError::CannotReturn);
                    }
                    exit_frame!(ExecOutcome::Returned(receiver));
                }
                RETURN_NIL => {
                    if let Some(home_frame_id) = block_home {
                        if self.active_frame_ids.contains(&home_frame_id) {
                            exit_frame!(ExecOutcome::NonLocalReturn {
                                home_frame_id,
                                result: Oop::nil(),
                            });
                        }
                        return Err(VmError::CannotReturn);
                    }
                    exit_frame!(ExecOutcome::Returned(Oop::nil()));
                }
                BLOCK_RETURN => {
                    let result = self.stack.peek().unwrap_or(Oop::nil());
                    exit_frame!(ExecOutcome::Returned(result));
                }
                _ => {
                    return Err(VmError::InvalidOpcode {
                        method,
                        ip: ip - 1,
                        opcode,
                    });
                }
            }
        }

        let result = self.stack.peek().unwrap_or(Oop::nil());
        self.active_frame_ids.pop();
        self.stack.truncate(frame_base);
        self.stack.fp = previous_fp;
        Ok(ExecOutcome::Returned(result))
    }

    fn try_fast_special_send(
        &mut self,
        special_index: usize,
        receiver: Oop,
        arg: Oop,
    ) -> Result<Option<Oop>, VmError> {
        let result = match special_index {
            0 => receiver.checked_add_small_int(arg),
            1 => receiver.checked_sub_small_int(arg),
            2 => receiver.checked_mul_small_int(arg),
            3 => receiver.checked_div_small_int(arg),
            4 => receiver
                .small_int_compare(arg, |a, b| a < b)
                .map(|v| self.boolean_oop(v)),
            5 => receiver
                .small_int_compare(arg, |a, b| a > b)
                .map(|v| self.boolean_oop(v)),
            6 => receiver
                .small_int_compare(arg, |a, b| a <= b)
                .map(|v| self.boolean_oop(v)),
            7 => receiver
                .small_int_compare(arg, |a, b| a >= b)
                .map(|v| self.boolean_oop(v)),
            8 => receiver
                .small_int_compare(arg, |a, b| a == b)
                .map(|v| self.boolean_oop(v)),
            9 => receiver
                .small_int_compare(arg, |a, b| a != b)
                .map(|v| self.boolean_oop(v)),
            10 => match (receiver.as_i64(), arg.as_i64()) {
                (Some(lhs), Some(rhs)) => Oop::from_i64(lhs & rhs),
                _ => None,
            },
            11 => match (receiver.as_i64(), arg.as_i64()) {
                (Some(lhs), Some(rhs)) => Oop::from_i64(lhs | rhs),
                _ => None,
            },
            12 => match arg.as_i64() {
                Some(shift) if shift >= 0 => receiver.checked_shl_small_int(arg),
                Some(shift) => receiver.checked_shr_small_int(Oop::from_i64(-shift).unwrap()),
                None => None,
            },
            14 => self.try_fast_at(receiver, arg)?,
            _ => None,
        };
        Ok(result)
    }

    fn try_fast_at(&mut self, receiver: Oop, index_oop: Oop) -> Result<Option<Oop>, VmError> {
        let index = match index_oop.as_i64() {
            Some(value) if value > 0 => value as usize - 1,
            _ => return Ok(None),
        };
        match self.class_of(receiver)? {
            CLASS_INDEX_ARRAY => {
                let size = self.heap.slot_count(receiver).unwrap_or(0);
                if index >= size {
                    return Ok(None);
                }
                Ok(self.heap.read_slot(receiver, index))
            }
            CLASS_INDEX_BYTE_ARRAY | CLASS_INDEX_STRING | CLASS_INDEX_SYMBOL => {
                let bytes = match self.heap.bytes(receiver) {
                    Some(bytes) => bytes,
                    None => return Ok(None),
                };
                if index >= bytes.len() {
                    return Ok(None);
                }
                Ok(Oop::from_i64(bytes[index] as i64))
            }
            _ => Ok(None),
        }
    }

    fn try_fast_at_put(
        &mut self,
        receiver: Oop,
        index_oop: Oop,
        value: Oop,
    ) -> Result<Option<Oop>, VmError> {
        let index = match index_oop.as_i64() {
            Some(v) if v > 0 => v as usize - 1,
            _ => return Ok(None),
        };
        if self.class_of(receiver)? != CLASS_INDEX_ARRAY {
            return Ok(None);
        }
        let size = self.heap.slot_count(receiver).unwrap_or(0);
        if index >= size {
            return Ok(None);
        }
        self.heap.write_slot(receiver, index, value);
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bootstrap,
        bytecode::{PUSH_CLOSURE, PUSH_ONE, PUSH_TWO, RETURN_TOP, SEND_SPECIAL_BASE},
        class_table::{CLASS_INDEX_ARRAY, CLASS_INDEX_BEHAVIOR, CLASS_INDEX_METHOD_CONTEXT},
        compiler::compile_doit,
        corelib,
        object::MethodHeaderFields,
    };

    use super::Vm;

    #[test]
    fn can_boot_vm() {
        let vm = Vm::new();
        assert!(vm.class_table.len() >= 19);
    }

    #[test]
    fn executes_simple_constant_method() {
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
    }

    #[test]
    fn closure_captures_outer_context_object() {
        let mut vm = Vm::new();
        let method = vm.compiled_method(
            MethodHeaderFields {
                num_args: 0,
                num_temps: 0,
                num_literals: 0,
                flags: 0,
            },
            &[],
            &[PUSH_ONE, PUSH_CLOSURE, 0, 1, 0, 1, RETURN_TOP, RETURN_TOP],
        );
        let closure = vm.run_method(method, vm.true_oop(), &[]).unwrap();
        let outer = vm.heap.read_slot(closure, 0).unwrap();
        assert_eq!(vm.class_of(outer).unwrap(), CLASS_INDEX_METHOD_CONTEXT);
    }

    #[test]
    fn vm_full_gc_collects_unreachable_heap_objects() {
        let mut vm = Vm::new();
        let before = vm.heap.all_objects().len();
        let _garbage = vm.make_array(&[crate::value::Oop::from_i64(1).unwrap()]);
        let after_alloc = vm.heap.all_objects().len();
        assert!(after_alloc > before);
        let result = vm.full_gc();
        assert!(result.collected >= 1);
        assert!(vm.heap.all_objects().len() < after_alloc);
    }

    #[test]
    fn bootstrapped_smalltalk_dictionary_exists() {
        let vm = Vm::new();
        let dictionary = vm.special_objects[bootstrap::SPECIAL_OBJECT_SMALLTALK];
        assert!(dictionary.is_heap_ptr());
        assert!(vm.heap.slot_count(dictionary).unwrap() >= vm.class_table.len() - 1);
    }

    #[test]
    fn method_dictionary_is_materialized_on_class_object() {
        let vm = Vm::new();
        let class_oop = vm.class_table.class_oop(CLASS_INDEX_ARRAY).unwrap();
        let method_dict = vm.heap.read_slot(class_oop, 1).unwrap();
        assert!(method_dict.is_heap_ptr());
        assert!(vm.heap.slot_count(method_dict).unwrap() >= 2);
    }

    #[test]
    fn primitive_subclass_creates_class_from_doit() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "Behavior subclass: #Point instanceVariableNames: 'x y'")
            .unwrap();
        let class_oop = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        let class_index = vm.class_table.class_index_of_oop(class_oop).unwrap();
        let info = vm.class_table.get(class_index).unwrap();
        assert_eq!(info.superclass, Some(CLASS_INDEX_BEHAVIOR));
        assert_eq!(info.instance_variables, vec!["x", "y"]);
        assert_eq!(vm.global_value("Point"), Some(class_oop));
    }

    #[test]
    fn primitive_extended_subclass_creates_class_from_standard_definition() {
        let mut vm = Vm::new();
        let method = compile_doit(
            &mut vm,
            "Behavior subclass: #Widget instanceVariableNames: 'left right' classVariableNames: '' poolDictionaries: '' category: 'Demo'",
        )
        .unwrap();
        let class_oop = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        let class_index = vm.class_table.class_index_of_oop(class_oop).unwrap();
        let info = vm.class_table.get(class_index).unwrap();
        assert_eq!(info.superclass, Some(CLASS_INDEX_BEHAVIOR));
        assert_eq!(info.instance_variables, vec!["left", "right"]);
    }

    #[test]
    fn primitive_class_returns_receiver_class() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "1 class").unwrap();
        let class_oop = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(vm.class_table.class_index_of_oop(class_oop), Some(crate::class_table::CLASS_INDEX_SMALL_INTEGER));
    }

    #[test]
    fn behavior_accessors_are_available() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "1 class name").unwrap();
        let symbol = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(vm.symbol_text(symbol).unwrap(), "SmallInteger");
    }

    #[test]
    fn core_library_source_mentions_conditionals() {
        assert!(corelib::SOURCE.contains("ifTrue:"));
        assert!(corelib::SOURCE.contains("whileTrue:"));
    }

    #[test]
    fn bootstrapped_if_true_if_false_works() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "true ifTrue: [ 1 ] ifFalse: [ 2 ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(1));
        let method = compile_doit(&mut vm, "false ifTrue: [ 1 ] ifFalse: [ 2 ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(2));
    }

    #[test]
    fn bootstrapped_while_true_and_to_do_work() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "Total := 0. 1 to: 5 do: [:i | Total := Total + i]. Total").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(15));
    }

    #[test]
    fn bootstrapped_is_nil_and_not_nil_work() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "nil isNil ifTrue: [ 1 ] ifFalse: [ 2 ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(1));
        let method = compile_doit(&mut vm, "1 notNil ifTrue: [ 3 ] ifFalse: [ 4 ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
    }

    #[test]
    fn bootstrapped_if_nil_variants_work() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "nil ifNil: [ 7 ] ifNotNil: [:x | x ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(7));
        let method = compile_doit(&mut vm, "3 ifNil: [ 7 ] ifNotNil: [:x | x + 1 ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(4));
    }

    #[test]
    fn bootstrapped_and_or_and_times_repeat_work() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "(true and: [ 1 < 2 ]) ifTrue: [ 10 ] ifFalse: [ 20 ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(10));
        let method = compile_doit(&mut vm, "(false or: [ 1 < 2 ]) ifTrue: [ 30 ] ifFalse: [ 40 ]").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(30));
        let method = compile_doit(&mut vm, "Count := 0. 3 timesRepeat: [ Count := Count + 1 ]. Count").unwrap();
        let result = vm.run_method(method, crate::value::Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
    }
}
