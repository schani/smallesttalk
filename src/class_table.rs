use std::collections::HashMap;

use crate::{object::Format, value::Oop};

pub const CLASS_INDEX_NONE: u32 = 0;
pub const CLASS_INDEX_UNDEFINED_OBJECT: u32 = 1;
pub const CLASS_INDEX_TRUE: u32 = 2;
pub const CLASS_INDEX_FALSE: u32 = 3;
pub const CLASS_INDEX_SMALL_INTEGER: u32 = 4;
pub const CLASS_INDEX_ARRAY: u32 = 5;
pub const CLASS_INDEX_BYTE_ARRAY: u32 = 6;
pub const CLASS_INDEX_STRING: u32 = 7;
pub const CLASS_INDEX_SYMBOL: u32 = 8;
pub const CLASS_INDEX_BLOCK_CLOSURE: u32 = 9;
pub const CLASS_INDEX_COMPILED_METHOD: u32 = 10;
pub const CLASS_INDEX_METHOD_CONTEXT: u32 = 11;
pub const CLASS_INDEX_ASSOCIATION: u32 = 12;
pub const CLASS_INDEX_METHOD_DICTIONARY: u32 = 13;
pub const CLASS_INDEX_CHARACTER: u32 = 14;
pub const CLASS_INDEX_FLOAT: u32 = 15;
pub const CLASS_INDEX_LARGE_POSITIVE_INTEGER: u32 = 16;
pub const CLASS_INDEX_MESSAGE: u32 = 17;
pub const CLASS_INDEX_BEHAVIOR: u32 = 18;

#[derive(Clone, Debug)]
pub struct ClassInfo {
    pub oop: Oop,
    pub name: String,
    pub superclass: Option<u32>,
    pub instance_format: Format,
    pub fixed_fields: usize,
    pub instance_variables: Vec<String>,
    pub methods: HashMap<Oop, Oop>,
}

#[derive(Clone, Debug, Default)]
pub struct ClassTable {
    classes: Vec<Option<ClassInfo>>,
}

impl ClassTable {
    pub fn new() -> Self {
        let mut classes = Vec::new();
        classes.resize_with(1, || None);
        Self { classes }
    }

    pub fn len(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.len() <= 1
    }

    fn ensure_index(&mut self, index: u32) {
        let wanted = index as usize + 1;
        if self.classes.len() < wanted {
            self.classes.resize_with(wanted, || None);
        }
    }

    pub fn insert_at(&mut self, index: u32, info: ClassInfo) {
        self.ensure_index(index);
        self.classes[index as usize] = Some(info);
    }

    pub fn add_class(&mut self, info: ClassInfo) -> u32 {
        let index = self.classes.len() as u32;
        self.classes.push(Some(info));
        index
    }

    pub fn get(&self, index: u32) -> Option<&ClassInfo> {
        self.classes.get(index as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut ClassInfo> {
        self.classes.get_mut(index as usize)?.as_mut()
    }

    pub fn class_oop(&self, index: u32) -> Option<Oop> {
        self.get(index).map(|info| info.oop)
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &ClassInfo)> {
        self.classes
            .iter()
            .enumerate()
            .filter_map(|(index, info)| info.as_ref().map(|info| (index as u32, info)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (u32, &mut ClassInfo)> {
        self.classes
            .iter_mut()
            .enumerate()
            .filter_map(|(index, info)| info.as_mut().map(|info| (index as u32, info)))
    }

    pub fn class_index_of_oop(&self, oop: Oop) -> Option<u32> {
        self.iter()
            .find_map(|(index, info)| (info.oop == oop).then_some(index))
    }

    pub fn superclass_of(&self, index: u32) -> Option<u32> {
        self.get(index)?.superclass
    }

    pub fn instance_variable_index(&self, class_index: u32, name: &str) -> Option<usize> {
        self.get(class_index)?
            .instance_variables
            .iter()
            .position(|ivar| ivar == name)
    }

    pub fn set_method(&mut self, index: u32, selector: Oop, method: Oop) {
        if let Some(info) = self.get_mut(index) {
            info.methods.insert(selector, method);
        }
    }

    pub fn lookup_method(&self, start_class: u32, selector: Oop) -> Option<(u32, Oop)> {
        let mut current = Some(start_class);
        while let Some(class_index) = current {
            let info = self.get(class_index)?;
            if let Some(method) = info.methods.get(&selector) {
                return Some((class_index, *method));
            }
            current = info.superclass;
        }
        None
    }
}

pub fn encode_format_descriptor(format: Format, fixed_fields: usize) -> Oop {
    let raw = ((format as u64) << 16) | (fixed_fields as u64 & 0xffff);
    Oop::from_i64(raw as i64).expect("format descriptor must fit SmallInteger")
}

pub fn decode_format_descriptor(descriptor: Oop) -> Option<(Format, usize)> {
    let raw = descriptor.as_i64()? as u64;
    let format = Format::from_u8(((raw >> 16) & 0xff) as u8)?;
    let fixed_fields = (raw & 0xffff) as usize;
    Some((format, fixed_fields))
}
