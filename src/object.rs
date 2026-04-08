#![allow(unsafe_op_in_unsafe_fn)]

use crate::value::Oop;

pub const HEADER_SIZE_WORDS: usize = 1;
pub const OVERFLOW_SIZE_SENTINEL: usize = 0x0fff;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Empty = 0,
    FixedPointers = 1,
    VarPointers = 2,
    FixedAndVar = 3,
    Weak = 4,
    Words = 6,
    Bytes8 = 8,
    Bytes16 = 9,
    Bytes24 = 10,
    Bytes32 = 11,
    CompiledMethod = 12,
}

impl Format {
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Empty,
            1 => Self::FixedPointers,
            2 => Self::VarPointers,
            3 => Self::FixedAndVar,
            4 => Self::Weak,
            6 => Self::Words,
            8 => Self::Bytes8,
            9 => Self::Bytes16,
            10 => Self::Bytes24,
            11 => Self::Bytes32,
            12 => Self::CompiledMethod,
            _ => return None,
        })
    }

    #[inline]
    pub fn is_pointer_format(self) -> bool {
        matches!(
            self,
            Self::FixedPointers | Self::VarPointers | Self::FixedAndVar | Self::Weak
        )
    }

    #[inline]
    pub fn is_byte_format(self) -> bool {
        matches!(
            self,
            Self::Bytes8 | Self::Bytes16 | Self::Bytes24 | Self::Bytes32
        )
    }

    #[inline]
    pub fn padding_bytes(self) -> usize {
        match self {
            Self::Bytes8 => 0,
            Self::Bytes16 => 1,
            Self::Bytes24 => 2,
            Self::Bytes32 => 3,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderWord(u64);

impl HeaderWord {
    #[inline]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub fn new(class_index: u32, format: Format, id_hash: u32, gc_bits: u8, size: usize) -> Self {
        let size_field = if size >= OVERFLOW_SIZE_SENTINEL {
            OVERFLOW_SIZE_SENTINEL as u64
        } else {
            size as u64
        };
        let raw = ((class_index as u64 & 0x00ff_ffff) << 40)
            | ((format as u64 & 0x0f) << 36)
            | ((id_hash as u64 & 0x003f_ffff) << 14)
            | ((gc_bits as u64 & 0x03) << 12)
            | size_field;
        Self(raw)
    }

    #[inline]
    pub fn raw(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn class_index(self) -> u32 {
        ((self.0 >> 40) & 0x00ff_ffff) as u32
    }

    #[inline]
    pub fn format(self) -> Format {
        Format::from_u8(((self.0 >> 36) & 0x0f) as u8).expect("invalid object format")
    }

    #[inline]
    pub fn id_hash(self) -> u32 {
        ((self.0 >> 14) & 0x003f_ffff) as u32
    }

    #[inline]
    pub fn gc_bits(self) -> u8 {
        ((self.0 >> 12) & 0x03) as u8
    }

    #[inline]
    pub fn size_field(self) -> usize {
        (self.0 & 0x0fff) as usize
    }

    #[inline]
    pub fn has_overflow_size(self) -> bool {
        self.size_field() == OVERFLOW_SIZE_SENTINEL
    }
}

/// View into a heap object. Not owning — the heap owns the memory.
#[derive(Clone, Copy)]
pub struct ObjHeader(*mut u64);

impl ObjHeader {
    #[inline]
    pub unsafe fn from_oop(oop: Oop) -> Option<Self> {
        oop.as_ptr::<u64>().map(Self)
    }

    #[inline]
    pub fn as_oop(self) -> Oop {
        Oop::from_ptr(self.0)
    }

    #[inline]
    pub fn raw_ptr(self) -> *mut u64 {
        self.0
    }

    #[inline]
    pub unsafe fn header_word(self) -> HeaderWord {
        HeaderWord(*self.0)
    }

    #[inline]
    pub unsafe fn set_header_word(self, header: HeaderWord) {
        *self.0 = header.raw();
    }

    #[inline]
    pub unsafe fn class_index(self) -> u32 {
        self.header_word().class_index()
    }

    #[inline]
    pub unsafe fn format(self) -> Format {
        self.header_word().format()
    }

    #[inline]
    pub unsafe fn slot_count(self) -> usize {
        let header = self.header_word();
        if header.has_overflow_size() {
            *self.0.sub(1) as usize
        } else {
            header.size_field()
        }
    }

    #[inline]
    pub unsafe fn slot(self, index: usize) -> Oop {
        debug_assert!(index < self.slot_count());
        Oop::from_raw(*self.0.add(1 + index))
    }

    #[inline]
    pub unsafe fn set_slot(self, index: usize, value: Oop) {
        debug_assert!(index < self.slot_count());
        *self.0.add(1 + index) = value.raw();
    }

    #[inline]
    pub unsafe fn word(self, index: usize) -> u64 {
        debug_assert!(index < self.slot_count());
        *self.0.add(1 + index)
    }

    #[inline]
    pub unsafe fn set_word(self, index: usize, value: u64) {
        debug_assert!(index < self.slot_count());
        *self.0.add(1 + index) = value;
    }

    #[inline]
    pub unsafe fn body_words(self) -> *mut u64 {
        self.0.add(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MethodHeaderFields {
    pub num_args: u8,
    pub num_temps: u8,
    pub num_literals: u16,
    pub flags: u32,
}

impl MethodHeaderFields {
    pub fn encode(self) -> Oop {
        let raw = ((self.num_args as u64 & 0x7f) << 56)
            | ((self.num_temps as u64) << 48)
            | ((self.num_literals as u64) << 32)
            | (self.flags as u64);
        Oop::from_raw((raw << 1) | 1)
    }

    pub fn decode(oop: Oop) -> Option<Self> {
        let raw = oop.as_i64()? as u64;
        Some(Self {
            num_args: ((raw >> 56) & 0x7f) as u8,
            num_temps: ((raw >> 48) & 0xff) as u8,
            num_literals: ((raw >> 32) & 0xffff) as u16,
            flags: raw as u32,
        })
    }

    #[inline]
    pub fn primitive_index(self) -> u16 {
        (self.flags & 0x03ff) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, HeaderWord, MethodHeaderFields, OVERFLOW_SIZE_SENTINEL};

    #[test]
    fn header_roundtrip() {
        let header = HeaderWord::new(17, Format::VarPointers, 42, 2, 33);
        assert_eq!(header.class_index(), 17);
        assert_eq!(header.format(), Format::VarPointers);
        assert_eq!(header.id_hash(), 42);
        assert_eq!(header.gc_bits(), 2);
        assert_eq!(header.size_field(), 33);
    }

    #[test]
    fn overflow_header_uses_sentinel() {
        let header = HeaderWord::new(1, Format::FixedPointers, 0, 0, OVERFLOW_SIZE_SENTINEL + 1);
        assert!(header.has_overflow_size());
    }

    #[test]
    fn method_header_roundtrip() {
        let fields = MethodHeaderFields {
            num_args: 2,
            num_temps: 3,
            num_literals: 17,
            flags: 99,
        };
        assert_eq!(MethodHeaderFields::decode(fields.encode()), Some(fields));
    }
}
