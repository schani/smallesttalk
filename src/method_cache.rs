use crate::value::Oop;

pub const CACHE_SIZE: usize = 2048;

#[derive(Clone, Copy, Debug)]
struct CacheEntry {
    class_index: u32,
    selector: Oop,
    method: Oop,
}

impl CacheEntry {
    const EMPTY: Self = Self {
        class_index: 0,
        selector: Oop::nil(),
        method: Oop::nil(),
    };
}

#[derive(Clone, Debug)]
pub struct MethodCache {
    entries: Vec<CacheEntry>,
}

impl Default for MethodCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MethodCache {
    pub fn new() -> Self {
        Self {
            entries: vec![CacheEntry::EMPTY; CACHE_SIZE],
        }
    }

    #[inline]
    pub fn cache_index(class_index: u32, selector: Oop) -> usize {
        let h = (class_index as u64) ^ selector.raw();
        ((h as usize) >> 2) & (CACHE_SIZE - 1)
    }

    pub fn lookup(&self, class_index: u32, selector: Oop) -> Option<Oop> {
        let entry = self.entries[Self::cache_index(class_index, selector)];
        (entry.class_index == class_index && entry.selector == selector).then_some(entry.method)
    }

    pub fn insert(&mut self, class_index: u32, selector: Oop, method: Oop) {
        let idx = Self::cache_index(class_index, selector);
        self.entries[idx] = CacheEntry {
            class_index,
            selector,
            method,
        };
    }

    pub fn clear(&mut self) {
        self.entries.fill(CacheEntry::EMPTY);
    }
}
