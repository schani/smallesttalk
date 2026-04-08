pub const PRIMITIVE_NONE: u16 = 0;
pub const PRIMITIVE_BASIC_NEW: u16 = 1;
pub const PRIMITIVE_AT: u16 = 2;
pub const PRIMITIVE_AT_PUT: u16 = 3;
pub const PRIMITIVE_SUBCLASS: u16 = 4;
pub const PRIMITIVE_THIS_CONTEXT: u16 = 5;
pub const PRIMITIVE_CLASS: u16 = 6;
pub const PRIMITIVE_SUBCLASS_EXTENDED: u16 = 7;
pub const PRIMITIVE_SIZE: u16 = 8;
pub const PRIMITIVE_BASIC_NEW_SIZED: u16 = 9;
pub const PRIMITIVE_COPY_FROM_TO: u16 = 10;
pub const PRIMITIVE_INSTALL_METHOD: u16 = 11;
pub const PRIMITIVE_GLOBAL_ASSOCIATION: u16 = 12;
pub const PRIMITIVE_INSTANCE_VARIABLE_INDEX: u16 = 13;
pub const PRIMITIVE_COMPILED_METHOD: u16 = 14;
pub const PRIMITIVE_EQUALS: u16 = 15;
pub const PRIMITIVE_INTERN_SYMBOL: u16 = 16;
pub const PRIMITIVE_INSTALL_COMPILED_METHOD: u16 = 17;

#[inline]
pub fn primitive_index(flags: u32) -> u16 {
    (flags & 0x03ff) as u16
}
