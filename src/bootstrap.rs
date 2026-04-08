use std::collections::HashMap;

use crate::{
    bytecode::{
        DUP, PUSH_INST_VAR_BASE, PUSH_TEMP_BASE, RETURN_NIL, RETURN_TOP, SPECIAL_SEND_SELECTORS,
        POP_STORE_INST_VAR_BASE,
    },
    class_table::{
        CLASS_INDEX_ARRAY, CLASS_INDEX_ASSOCIATION, CLASS_INDEX_BEHAVIOR,
        CLASS_INDEX_BLOCK_CLOSURE, CLASS_INDEX_BYTE_ARRAY, CLASS_INDEX_CHARACTER,
        CLASS_INDEX_COMPILED_METHOD, CLASS_INDEX_FALSE, CLASS_INDEX_FLOAT,
        CLASS_INDEX_LARGE_POSITIVE_INTEGER, CLASS_INDEX_MESSAGE, CLASS_INDEX_METHOD_CONTEXT,
        CLASS_INDEX_METHOD_DICTIONARY, CLASS_INDEX_SMALL_INTEGER, CLASS_INDEX_STRING,
        CLASS_INDEX_SYMBOL, CLASS_INDEX_TRUE, CLASS_INDEX_UNDEFINED_OBJECT, ClassInfo, ClassTable,
        encode_format_descriptor,
    },
    heap::{Generation, Heap},
    object::{Format, MethodHeaderFields},
    primitives::{
        PRIMITIVE_AT, PRIMITIVE_AT_PUT, PRIMITIVE_BASIC_NEW, PRIMITIVE_BASIC_NEW_SIZED,
        PRIMITIVE_CLASS, PRIMITIVE_COMPILED_METHOD, PRIMITIVE_COPY_FROM_TO,
        PRIMITIVE_EQUALS, PRIMITIVE_GLOBAL_ASSOCIATION, PRIMITIVE_INSTALL_COMPILED_METHOD,
        PRIMITIVE_INSTALL_METHOD, PRIMITIVE_INSTANCE_VARIABLE_INDEX, PRIMITIVE_INTERN_SYMBOL,
        PRIMITIVE_SIZE, PRIMITIVE_SUBCLASS, PRIMITIVE_SUBCLASS_EXTENDED,
        PRIMITIVE_THIS_CONTEXT,
    },
    value::Oop,
};

pub const SPECIAL_OBJECT_NIL: usize = 0;
pub const SPECIAL_OBJECT_TRUE: usize = 1;
pub const SPECIAL_OBJECT_FALSE: usize = 2;
pub const SPECIAL_OBJECT_SMALLTALK: usize = 3;
pub const SPECIAL_OBJECT_SYMBOL_TABLE: usize = 4;
pub const SPECIAL_OBJECT_DNU_SELECTOR: usize = 5;
pub const SPECIAL_OBJECT_CANNOT_RETURN_SELECTOR: usize = 6;
pub const SPECIAL_OBJECT_CLASS_TABLE_ARRAY: usize = 7;

pub struct BootstrapState {
    pub heap: Heap,
    pub class_table: ClassTable,
    pub special_objects: Vec<Oop>,
    pub symbols: HashMap<String, Oop>,
    pub globals: HashMap<String, Oop>,
    pub special_selectors: Vec<Oop>,
}

fn well_known_classes() -> Vec<(u32, &'static str, Option<u32>, Format, usize, &'static [&'static str])> {
    vec![
        (
            CLASS_INDEX_UNDEFINED_OBJECT,
            "UndefinedObject",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Empty,
            0,
            &[],
        ),
        (
            CLASS_INDEX_TRUE,
            "True",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Empty,
            0,
            &[],
        ),
        (
            CLASS_INDEX_FALSE,
            "False",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Empty,
            0,
            &[],
        ),
        (
            CLASS_INDEX_SMALL_INTEGER,
            "SmallInteger",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Empty,
            0,
            &[],
        ),
        (
            CLASS_INDEX_ARRAY,
            "Array",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::VarPointers,
            0,
            &[],
        ),
        (
            CLASS_INDEX_BYTE_ARRAY,
            "ByteArray",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Bytes8,
            0,
            &[],
        ),
        (
            CLASS_INDEX_STRING,
            "String",
            Some(CLASS_INDEX_BYTE_ARRAY),
            Format::Bytes8,
            0,
            &[],
        ),
        (
            CLASS_INDEX_SYMBOL,
            "Symbol",
            Some(CLASS_INDEX_STRING),
            Format::Bytes8,
            0,
            &[],
        ),
        (
            CLASS_INDEX_BLOCK_CLOSURE,
            "BlockClosure",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::FixedPointers,
            4,
            &["outer_context", "start_ip", "num_args", "method"],
        ),
        (
            CLASS_INDEX_COMPILED_METHOD,
            "CompiledMethod",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::CompiledMethod,
            0,
            &[],
        ),
        (
            CLASS_INDEX_METHOD_CONTEXT,
            "MethodContext",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::FixedPointers,
            6,
            &["saved_fp", "saved_ip", "method", "receiver", "frame_id", "frame_values"],
        ),
        (
            CLASS_INDEX_ASSOCIATION,
            "Association",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::FixedPointers,
            2,
            &["key", "value"],
        ),
        (
            CLASS_INDEX_METHOD_DICTIONARY,
            "MethodDictionary",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::VarPointers,
            0,
            &[],
        ),
        (
            CLASS_INDEX_CHARACTER,
            "Character",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Empty,
            0,
            &[],
        ),
        (
            CLASS_INDEX_FLOAT,
            "Float",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Words,
            1,
            &[],
        ),
        (
            CLASS_INDEX_LARGE_POSITIVE_INTEGER,
            "LargePositiveInteger",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::Words,
            0,
            &[],
        ),
        (
            CLASS_INDEX_MESSAGE,
            "Message",
            Some(CLASS_INDEX_BEHAVIOR),
            Format::FixedPointers,
            2,
            &["selector", "arguments"],
        ),
        (
            CLASS_INDEX_BEHAVIOR,
            "Behavior",
            None,
            Format::FixedPointers,
            6,
            &[
                "superclass",
                "method_dictionary",
                "format_descriptor",
                "instance_variable_names",
                "name",
                "subclasses",
            ],
        ),
    ]
}

fn intern_symbol(heap: &mut Heap, symbols: &mut HashMap<String, Oop>, text: &str) -> Oop {
    if let Some(existing) = symbols.get(text) {
        return *existing;
    }
    let oop = heap.allocate_bytes_in(CLASS_INDEX_SYMBOL, text.as_bytes(), Generation::Old);
    symbols.insert(text.to_string(), oop);
    oop
}

fn make_array(heap: &mut Heap, values: &[Oop]) -> Oop {
    let array = heap.allocate_object_in(
        CLASS_INDEX_ARRAY,
        Format::VarPointers,
        values.len(),
        Generation::Old,
    );
    for (index, value) in values.iter().copied().enumerate() {
        heap.write_slot(array, index, value);
    }
    array
}

fn install_method(
    heap: &mut Heap,
    class_table: &mut ClassTable,
    symbols: &mut HashMap<String, Oop>,
    class_index: u32,
    selector_text: &str,
    num_args: u8,
    primitive_index: u16,
    bytecodes: &[u8],
) {
    let selector = intern_symbol(heap, symbols, selector_text);
    let method = heap.allocate_compiled_method_in(
        CLASS_INDEX_COMPILED_METHOD,
        MethodHeaderFields {
            num_args,
            num_temps: 0,
            num_literals: 0,
            flags: primitive_index as u32,
        },
        &[],
        bytecodes,
        Generation::Old,
    );
    class_table.set_method(class_index, selector, method);
}

pub(crate) fn install_bootstrap_methods(
    heap: &mut Heap,
    class_table: &mut ClassTable,
    symbols: &mut HashMap<String, Oop>,
) {
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "new",
        0,
        PRIMITIVE_BASIC_NEW,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "new:",
        1,
        PRIMITIVE_BASIC_NEW_SIZED,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "class",
        0,
        PRIMITIVE_CLASS,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "=",
        1,
        PRIMITIVE_EQUALS,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "size",
        0,
        PRIMITIVE_SIZE,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "yourself",
        0,
        0,
        &[crate::bytecode::PUSH_SELF, RETURN_TOP],
    );
    for (selector, index) in [
        ("superclass", 0u8),
        ("methodDictionary", 1u8),
        ("formatDescriptor", 2u8),
        ("instanceVariableNames", 3u8),
        ("name", 4u8),
        ("subclasses", 5u8),
    ] {
        install_method(
            heap,
            class_table,
            symbols,
            CLASS_INDEX_BEHAVIOR,
            selector,
            0,
            0,
            &[PUSH_INST_VAR_BASE + index, RETURN_TOP],
        );
    }
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "subclass:instanceVariableNames:",
        2,
        PRIMITIVE_SUBCLASS,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "subclass:instanceVariableNames:classVariableNames:poolDictionaries:category:",
        5,
        PRIMITIVE_SUBCLASS_EXTENDED,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "compiledMethod:literals:bytecodes:numArgs:numTemps:",
        5,
        PRIMITIVE_COMPILED_METHOD,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "installMethod:literals:bytecodes:numArgs:numTemps:",
        5,
        PRIMITIVE_INSTALL_METHOD,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "installCompiledMethod:selector:",
        2,
        PRIMITIVE_INSTALL_COMPILED_METHOD,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "globalAssociation:",
        1,
        PRIMITIVE_GLOBAL_ASSOCIATION,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "instanceVariableIndex:",
        1,
        PRIMITIVE_INSTANCE_VARIABLE_INDEX,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "internSymbol:",
        1,
        PRIMITIVE_INTERN_SYMBOL,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "thisContext",
        0,
        PRIMITIVE_THIS_CONTEXT,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "doesNotUnderstand:",
        1,
        0,
        &[RETURN_NIL],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_BEHAVIOR,
        "cannotReturn:",
        1,
        0,
        &[RETURN_NIL],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_ARRAY,
        "at:",
        1,
        PRIMITIVE_AT,
        &[RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_ARRAY,
        "at:put:",
        2,
        PRIMITIVE_AT_PUT,
        &[RETURN_TOP],
    );
    for class_index in [
        CLASS_INDEX_BYTE_ARRAY,
        CLASS_INDEX_STRING,
        CLASS_INDEX_SYMBOL,
    ] {
        install_method(
            heap,
            class_table,
            symbols,
            class_index,
            "at:",
            1,
            PRIMITIVE_AT,
            &[RETURN_TOP],
        );
        install_method(
            heap,
            class_table,
            symbols,
            class_index,
            "at:put:",
            2,
            PRIMITIVE_AT_PUT,
            &[RETURN_TOP],
        );
        install_method(
            heap,
            class_table,
            symbols,
            class_index,
            "copyFrom:to:",
            2,
            PRIMITIVE_COPY_FROM_TO,
            &[RETURN_TOP],
        );
    }
    for (selector, argc) in [("value", 0), ("value:", 1), ("value:value:", 2)] {
        install_method(
            heap,
            class_table,
            symbols,
            CLASS_INDEX_BLOCK_CLOSURE,
            selector,
            argc,
            0,
            &[RETURN_NIL],
        );
    }
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_ASSOCIATION,
        "key",
        0,
        0,
        &[PUSH_INST_VAR_BASE, RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_ASSOCIATION,
        "value",
        0,
        0,
        &[PUSH_INST_VAR_BASE + 1, RETURN_TOP],
    );
    install_method(
        heap,
        class_table,
        symbols,
        CLASS_INDEX_ASSOCIATION,
        "value:",
        1,
        0,
        &[PUSH_TEMP_BASE, DUP, POP_STORE_INST_VAR_BASE + 1, RETURN_TOP],
    );
}

pub fn build() -> BootstrapState {
    let mut heap = Heap::new();
    let mut class_table = ClassTable::new();
    let mut symbols = HashMap::new();

    for (index, name, superclass, format, fixed_fields, instance_variables) in well_known_classes() {
        let class_oop = heap.allocate_object_in(
            CLASS_INDEX_BEHAVIOR,
            Format::FixedPointers,
            6,
            Generation::Old,
        );
        class_table.insert_at(
            index,
            ClassInfo {
                oop: class_oop,
                name: name.to_string(),
                superclass,
                instance_format: format,
                fixed_fields,
                instance_variables: instance_variables.iter().map(|name| (*name).to_string()).collect(),
                methods: HashMap::new(),
            },
        );
    }

    let empty_array = make_array(&mut heap, &[]);
    let empty_method_dict = heap.allocate_object_in(
        CLASS_INDEX_METHOD_DICTIONARY,
        Format::VarPointers,
        0,
        Generation::Old,
    );

    for (index, name, superclass, format, fixed_fields, instance_variables) in well_known_classes() {
        let class_oop = class_table.class_oop(index).unwrap();
        let name_symbol = intern_symbol(&mut heap, &mut symbols, name);
        heap.write_slot(
            class_oop,
            0,
            superclass
                .and_then(|class_index| class_table.class_oop(class_index))
                .unwrap_or_else(Oop::nil),
        );
        heap.write_slot(class_oop, 1, empty_method_dict);
        heap.write_slot(class_oop, 2, encode_format_descriptor(format, fixed_fields));
        let ivar_symbols = instance_variables
            .iter()
            .map(|name| intern_symbol(&mut heap, &mut symbols, name))
            .collect::<Vec<_>>();
        let ivar_array = make_array(&mut heap, &ivar_symbols);
        heap.write_slot(class_oop, 3, ivar_array);
        heap.write_slot(class_oop, 4, name_symbol);
        heap.write_slot(class_oop, 5, empty_array);
    }

    let true_oop = heap.allocate_object_in(CLASS_INDEX_TRUE, Format::Empty, 0, Generation::Old);
    let false_oop = heap.allocate_object_in(CLASS_INDEX_FALSE, Format::Empty, 0, Generation::Old);

    let dnu_selector = intern_symbol(&mut heap, &mut symbols, "doesNotUnderstand:");
    let cannot_return_selector = intern_symbol(&mut heap, &mut symbols, "cannotReturn:");

    let special_selectors = SPECIAL_SEND_SELECTORS
        .iter()
        .map(|selector| intern_symbol(&mut heap, &mut symbols, selector))
        .collect::<Vec<_>>();

    install_bootstrap_methods(&mut heap, &mut class_table, &mut symbols);

    let class_values = (1..class_table.len() as u32)
        .filter_map(|index| class_table.class_oop(index))
        .collect::<Vec<_>>();
    let class_table_array = make_array(&mut heap, &class_values);

    let symbol_values = symbols.values().copied().collect::<Vec<_>>();
    let symbol_table = make_array(&mut heap, &symbol_values);

    let special_objects = vec![
        Oop::nil(),
        true_oop,
        false_oop,
        Oop::nil(),
        symbol_table,
        dnu_selector,
        cannot_return_selector,
        class_table_array,
    ];

    BootstrapState {
        heap,
        class_table,
        special_objects,
        symbols,
        globals: HashMap::new(),
        special_selectors,
    }
}
