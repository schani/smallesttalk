use core::fmt;

const TAG_MASK: u64 = 1;
const SMALL_INT_TAG: u64 = 1;
const SMALL_INT_MIN: i64 = -(1i64 << 62);
const SMALL_INT_MAX: i64 = (1i64 << 62) - 1;

/// The universal Smalltalk value — a tagged 64-bit word.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Oop(pub(crate) u64);

impl Oop {
    #[inline]
    pub const fn nil() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_small_int(self) -> bool {
        (self.0 & TAG_MASK) == SMALL_INT_TAG
    }

    #[inline]
    pub const fn is_heap_ptr(self) -> bool {
        self.0 != 0 && (self.0 & TAG_MASK) == 0
    }

    #[inline]
    pub fn from_i64(value: i64) -> Option<Self> {
        if !(SMALL_INT_MIN..=SMALL_INT_MAX).contains(&value) {
            return None;
        }
        Some(Self((((value as i128) << 1) as i64 as u64) | SMALL_INT_TAG))
    }

    #[inline]
    pub fn from_usize(value: usize) -> Option<Self> {
        Self::from_i64(value as i64)
    }

    #[inline]
    pub fn as_i64(self) -> Option<i64> {
        self.is_small_int().then(|| (self.0 as i64) >> 1)
    }

    #[inline]
    pub fn expect_i64(self) -> i64 {
        self.as_i64().expect("expected SmallInteger")
    }

    #[inline]
    pub fn checked_add_small_int(self, other: Self) -> Option<Self> {
        Some(Self::from_i64(
            self.as_i64()?.checked_add(other.as_i64()?)?,
        )?)
    }

    #[inline]
    pub fn checked_sub_small_int(self, other: Self) -> Option<Self> {
        Some(Self::from_i64(
            self.as_i64()?.checked_sub(other.as_i64()?)?,
        )?)
    }

    #[inline]
    pub fn checked_mul_small_int(self, other: Self) -> Option<Self> {
        Some(Self::from_i64(
            self.as_i64()?.checked_mul(other.as_i64()?)?,
        )?)
    }

    #[inline]
    pub fn checked_div_small_int(self, other: Self) -> Option<Self> {
        let lhs = self.as_i64()?;
        let rhs = other.as_i64()?;
        if rhs == 0 || lhs % rhs != 0 {
            return None;
        }
        Self::from_i64(lhs / rhs)
    }

    #[inline]
    pub fn small_int_compare<F>(self, other: Self, pred: F) -> Option<bool>
    where
        F: FnOnce(i64, i64) -> bool,
    {
        Some(pred(self.as_i64()?, other.as_i64()?))
    }

    #[inline]
    pub fn checked_shl_small_int(self, amount: Self) -> Option<Self> {
        let value = self.as_i64()?;
        let amount = amount.as_i64()?;
        if !(0..63).contains(&amount) {
            return None;
        }
        Self::from_i64(value.checked_shl(amount as u32)?)
    }

    #[inline]
    pub fn checked_shr_small_int(self, amount: Self) -> Option<Self> {
        let value = self.as_i64()?;
        let amount = amount.as_i64()?;
        if !(0..63).contains(&amount) {
            return None;
        }
        Self::from_i64(value >> amount)
    }

    #[inline]
    pub fn from_ptr<T>(ptr: *mut T) -> Self {
        debug_assert!(!ptr.is_null());
        debug_assert_eq!((ptr as usize) & 1, 0);
        Self(ptr as usize as u64)
    }

    #[inline]
    pub fn as_ptr<T>(self) -> Option<*mut T> {
        self.is_heap_ptr().then_some(self.0 as usize as *mut T)
    }
}

impl fmt::Debug for Oop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            write!(f, "nil")
        } else if let Some(value) = self.as_i64() {
            write!(f, "SmallInteger({value})")
        } else {
            write!(f, "Oop(0x{:016x})", self.0)
        }
    }
}

impl fmt::Display for Oop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::Oop;

    #[test]
    fn small_int_roundtrip() {
        let oop = Oop::from_i64(-123).unwrap();
        assert!(oop.is_small_int());
        assert_eq!(oop.as_i64(), Some(-123));
    }

    #[test]
    fn nil_is_not_smallint() {
        assert!(Oop::nil().is_nil());
        assert!(!Oop::nil().is_small_int());
    }
}
