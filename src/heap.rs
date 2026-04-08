use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    object::{Format, HeaderWord, MethodHeaderFields, OVERFLOW_SIZE_SENTINEL, ObjHeader},
    value::Oop,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    Young,
    Old,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GcKind {
    Minor,
    Full,
}

#[derive(Clone, Debug, Default)]
pub struct GcResult {
    pub relocated: HashMap<u64, Oop>,
    pub collected: usize,
    pub survivors: usize,
    pub promoted: usize,
}

#[derive(Clone, Debug)]
pub struct ObjectSnapshot {
    pub oop: Oop,
    pub header_raw: u64,
    pub class_index: u32,
    pub format: Format,
    pub slot_count: usize,
    pub generation: Generation,
    pub byte_len: Option<usize>,
    pub payload_words: Vec<u64>,
}

#[derive(Clone, Debug)]
struct Allocation {
    storage: Box<[u64]>,
    header_index: usize,
    generation: Generation,
    byte_len: Option<usize>,
}

impl Allocation {
    fn object_ptr(&self) -> *mut u64 {
        unsafe { self.storage.as_ptr().add(self.header_index) as *mut u64 }
    }

    fn oop(&self) -> Oop {
        Oop::from_ptr(self.object_ptr())
    }

    fn header_raw(&self) -> u64 {
        self.storage[self.header_index]
    }

    fn header_word(&self) -> HeaderWord {
        HeaderWord::from_raw(self.header_raw())
    }

    fn slot_count(&self) -> usize {
        let header = self.header_word();
        if header.has_overflow_size() {
            self.storage[0] as usize
        } else {
            header.size_field()
        }
    }

    fn payload_words(&self) -> &[u64] {
        &self.storage[self.header_index + 1..self.header_index + 1 + self.slot_count()]
    }

    fn payload_words_mut(&mut self) -> &mut [u64] {
        let header_index = self.header_index;
        let slot_count = self.slot_count();
        &mut self.storage[header_index + 1..header_index + 1 + slot_count]
    }

    fn clone_with_generation(&self, generation: Generation) -> Self {
        Self {
            storage: self.storage.clone(),
            header_index: self.header_index,
            generation,
            byte_len: self.byte_len,
        }
    }
}

#[derive(Default, Debug)]
pub struct Heap {
    allocations: Vec<Allocation>,
    index_by_addr: HashMap<u64, usize>,
    dirty_cards: HashSet<usize>,
    next_hash: u32,
    nursery_limit_bytes: usize,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            allocations: Vec::new(),
            index_by_addr: HashMap::new(),
            dirty_cards: HashSet::new(),
            next_hash: 1,
            nursery_limit_bytes: 4 * 1024 * 1024,
        }
    }

    fn next_identity_hash(&mut self) -> u32 {
        let hash = self.next_hash & 0x003f_ffff;
        self.next_hash = self.next_hash.wrapping_add(1).max(1);
        hash.max(1)
    }

    fn rebuild_index(&mut self) {
        self.index_by_addr.clear();
        for (index, allocation) in self.allocations.iter().enumerate() {
            self.index_by_addr.insert(allocation.oop().raw(), index);
        }
    }

    fn store_allocation(
        &mut self,
        storage: Box<[u64]>,
        header_index: usize,
        generation: Generation,
        byte_len: Option<usize>,
    ) -> Oop {
        let allocation = Allocation {
            storage,
            header_index,
            generation,
            byte_len,
        };
        let oop = allocation.oop();
        let index = self.allocations.len();
        self.index_by_addr.insert(oop.raw(), index);
        self.allocations.push(allocation);
        oop
    }

    pub fn allocate_object(&mut self, class_index: u32, format: Format, slot_count: usize) -> Oop {
        self.allocate_object_in(class_index, format, slot_count, Generation::Young)
    }

    pub fn allocate_object_in(
        &mut self,
        class_index: u32,
        format: Format,
        slot_count: usize,
        generation: Generation,
    ) -> Oop {
        let header = HeaderWord::new(
            class_index,
            format,
            self.next_identity_hash(),
            0,
            slot_count,
        );
        self.allocate_from_raw_parts(header.raw(), slot_count, generation, None, None)
    }

    pub fn allocate_from_raw_parts(
        &mut self,
        header_raw: u64,
        slot_count: usize,
        generation: Generation,
        payload_words: Option<&[u64]>,
        byte_len: Option<usize>,
    ) -> Oop {
        let use_overflow = slot_count >= OVERFLOW_SIZE_SENTINEL;
        let total_words = 1 + slot_count + usize::from(use_overflow);
        let mut storage = vec![0u64; total_words].into_boxed_slice();
        let header_index = usize::from(use_overflow);
        if use_overflow {
            storage[0] = slot_count as u64;
        }
        storage[header_index] = header_raw;
        if let Some(payload_words) = payload_words {
            let payload = &mut storage[header_index + 1..header_index + 1 + slot_count];
            payload.copy_from_slice(payload_words);
        } else {
            let format = HeaderWord::from_raw(header_raw).format();
            if format.is_pointer_format() || format == Format::CompiledMethod {
                for word in storage.iter_mut().skip(header_index + 1) {
                    *word = Oop::nil().raw();
                }
            }
        }
        self.store_allocation(storage, header_index, generation, byte_len)
    }

    pub fn allocate_words_in(
        &mut self,
        class_index: u32,
        words: &[u64],
        generation: Generation,
    ) -> Oop {
        let oop = self.allocate_object_in(class_index, Format::Words, words.len(), generation);
        let header = self.header(oop).unwrap();
        unsafe {
            for (i, word) in words.iter().copied().enumerate() {
                header.set_word(i, word);
            }
        }
        oop
    }

    pub fn allocate_bytes_in(
        &mut self,
        class_index: u32,
        bytes: &[u8],
        generation: Generation,
    ) -> Oop {
        let slot_count = bytes.len().div_ceil(8);
        let padding = (slot_count * 8).saturating_sub(bytes.len());
        let format = match padding {
            0 | 4 | 5 | 6 | 7 => Format::Bytes8,
            1 => Format::Bytes16,
            2 => Format::Bytes24,
            3 => Format::Bytes32,
            _ => Format::Bytes8,
        };
        let oop = self.allocate_object_in(class_index, format, slot_count, generation);
        self.set_byte_len(oop, bytes.len());
        let header = self.header(oop).unwrap();
        unsafe {
            let ptr = header.body_words() as *mut u8;
            std::ptr::write_bytes(ptr, 0, slot_count * 8);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        }
        oop
    }

    pub fn allocate_compiled_method_in(
        &mut self,
        class_index: u32,
        header_fields: MethodHeaderFields,
        literals: &[Oop],
        bytecodes: &[u8],
        generation: Generation,
    ) -> Oop {
        let bytecode_words = bytecodes.len().div_ceil(8);
        let slot_count = 1 + literals.len() + bytecode_words;
        let oop =
            self.allocate_object_in(class_index, Format::CompiledMethod, slot_count, generation);
        self.set_byte_len(oop, bytecodes.len());
        let header = self.header(oop).unwrap();
        unsafe {
            header.set_slot(0, header_fields.encode());
            for (i, literal) in literals.iter().copied().enumerate() {
                header.set_slot(1 + i, literal);
            }
            let start = header.body_words().add(1 + literals.len()) as *mut u8;
            std::ptr::write_bytes(start, 0, bytecode_words * 8);
            std::ptr::copy_nonoverlapping(bytecodes.as_ptr(), start, bytecodes.len());
        }
        oop
    }

    pub fn header(&self, oop: Oop) -> Option<ObjHeader> {
        if !oop.is_heap_ptr() {
            return None;
        }
        unsafe { ObjHeader::from_oop(oop) }
    }

    fn allocation_index(&self, oop: Oop) -> Option<usize> {
        self.index_by_addr.get(&oop.raw()).copied()
    }

    fn allocation(&self, oop: Oop) -> Option<&Allocation> {
        self.allocation_index(oop)
            .map(|index| &self.allocations[index])
    }

    fn allocation_mut(&mut self, oop: Oop) -> Option<&mut Allocation> {
        let index = self.allocation_index(oop)?;
        self.allocations.get_mut(index)
    }

    pub fn object_format(&self, oop: Oop) -> Option<Format> {
        let header = self.header(oop)?;
        Some(unsafe { header.format() })
    }

    pub fn object_class_index(&self, oop: Oop) -> Option<u32> {
        let header = self.header(oop)?;
        Some(unsafe { header.class_index() })
    }

    pub fn slot_count(&self, oop: Oop) -> Option<usize> {
        let header = self.header(oop)?;
        Some(unsafe { header.slot_count() })
    }

    pub fn read_slot(&self, oop: Oop, index: usize) -> Option<Oop> {
        let header = self.header(oop)?;
        (index < unsafe { header.slot_count() }).then(|| unsafe { header.slot(index) })
    }

    pub fn write_slot(&mut self, target: Oop, slot: usize, value: Oop) -> Option<()> {
        let header = self.header(target)?;
        if slot >= unsafe { header.slot_count() } {
            return None;
        }
        unsafe {
            header.set_slot(slot, value);
        }
        if self.is_old(target) && self.is_young(value) {
            self.dirty_cards.insert((target.raw() as usize) / 512);
        }
        Some(())
    }

    pub fn read_word(&self, oop: Oop, index: usize) -> Option<u64> {
        let header = self.header(oop)?;
        (index < unsafe { header.slot_count() }).then(|| unsafe { header.word(index) })
    }

    pub fn write_word(&mut self, oop: Oop, index: usize, value: u64) -> Option<()> {
        let header = self.header(oop)?;
        if index >= unsafe { header.slot_count() } {
            return None;
        }
        unsafe {
            header.set_word(index, value);
        }
        Some(())
    }

    pub fn overwrite_header_raw(&mut self, oop: Oop, header_raw: u64) -> Option<()> {
        let header = self.header(oop)?;
        unsafe {
            header.set_header_word(HeaderWord::from_raw(header_raw));
        }
        Some(())
    }

    pub fn generation_of(&self, oop: Oop) -> Option<Generation> {
        Some(self.allocation(oop)?.generation)
    }

    pub fn is_young(&self, oop: Oop) -> bool {
        self.generation_of(oop) == Some(Generation::Young)
    }

    pub fn is_old(&self, oop: Oop) -> bool {
        self.generation_of(oop) == Some(Generation::Old)
    }

    pub fn dirty_card_count(&self) -> usize {
        self.dirty_cards.len()
    }

    fn strong_child_oops(&self, oop: Oop) -> Vec<Oop> {
        let Some(allocation) = self.allocation(oop) else {
            return Vec::new();
        };
        match allocation.header_word().format() {
            Format::FixedPointers | Format::VarPointers | Format::FixedAndVar => allocation
                .payload_words()
                .iter()
                .copied()
                .map(Oop::from_raw)
                .filter(|oop| oop.is_heap_ptr())
                .collect(),
            Format::Weak => Vec::new(),
            Format::CompiledMethod => {
                let Some(method_header) =
                    MethodHeaderFields::decode(Oop::from_raw(allocation.payload_words()[0]))
                else {
                    return Vec::new();
                };
                allocation
                    .payload_words()
                    .iter()
                    .take(1 + method_header.num_literals as usize)
                    .copied()
                    .map(Oop::from_raw)
                    .filter(|oop| oop.is_heap_ptr())
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn rewrite_payload_oops(
        allocation: &mut Allocation,
        relocated: &HashMap<u64, Oop>,
        retained: &HashSet<u64>,
    ) {
        let format = allocation.header_word().format();
        let payload = allocation.payload_words_mut();
        let oop_word_count = match format {
            Format::FixedPointers | Format::VarPointers | Format::FixedAndVar => payload.len(),
            Format::Weak => payload.len(),
            Format::CompiledMethod => MethodHeaderFields::decode(Oop::from_raw(payload[0]))
                .map(|header| 1 + header.num_literals as usize)
                .unwrap_or(0),
            _ => 0,
        };

        let is_weak = format == Format::Weak;
        for word in payload.iter_mut().take(oop_word_count) {
            let oop = Oop::from_raw(*word);
            if !oop.is_heap_ptr() {
                continue;
            }
            if let Some(new_oop) = relocated.get(&oop.raw()) {
                *word = new_oop.raw();
            } else if is_weak || !retained.contains(&oop.raw()) {
                *word = Oop::nil().raw();
            }
        }
    }

    fn trace_reachable(&self, roots: &[Oop]) -> HashSet<u64> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        for root in roots {
            if root.is_heap_ptr() && reachable.insert(root.raw()) {
                queue.push_back(*root);
            }
        }

        while let Some(oop) = queue.pop_front() {
            for child in self.strong_child_oops(oop) {
                if reachable.insert(child.raw()) {
                    queue.push_back(child);
                }
            }
        }

        reachable
    }

    pub fn collect_garbage(&mut self, roots: &[Oop], kind: GcKind) -> GcResult {
        let reachable = self.trace_reachable(roots);
        let retained = match kind {
            GcKind::Minor => self
                .allocations
                .iter()
                .filter(|allocation| allocation.generation == Generation::Old)
                .map(Allocation::oop)
                .map(|oop| oop.raw())
                .chain(reachable.iter().copied())
                .collect::<HashSet<_>>(),
            GcKind::Full => reachable,
        };

        let old_allocations = std::mem::take(&mut self.allocations);
        self.index_by_addr.clear();
        let mut new_allocations = Vec::new();
        let mut relocated = HashMap::new();
        let mut promoted = 0usize;
        let mut collected = 0usize;

        for allocation in old_allocations {
            let old_oop = allocation.oop();
            if !retained.contains(&old_oop.raw()) {
                collected += 1;
                continue;
            }

            let new_allocation = match kind {
                GcKind::Minor if allocation.generation == Generation::Old => allocation,
                GcKind::Minor => {
                    promoted += 1;
                    allocation.clone_with_generation(Generation::Old)
                }
                GcKind::Full => allocation.clone_with_generation(Generation::Old),
            };

            let new_oop = new_allocation.oop();
            relocated.insert(old_oop.raw(), new_oop);
            new_allocations.push(new_allocation);
        }

        for allocation in &mut new_allocations {
            Self::rewrite_payload_oops(allocation, &relocated, &retained);
        }

        self.allocations = new_allocations;
        self.rebuild_index();
        self.dirty_cards.clear();

        GcResult {
            relocated,
            collected,
            survivors: self.allocations.len(),
            promoted,
        }
    }

    pub fn minor_gc(&mut self, roots: &[Oop]) -> GcResult {
        self.collect_garbage(roots, GcKind::Minor)
    }

    pub fn full_gc(&mut self, roots: &[Oop]) -> GcResult {
        self.collect_garbage(roots, GcKind::Full)
    }

    pub fn byte_len(&self, oop: Oop) -> Option<usize> {
        let allocation = self.allocation(oop)?;
        allocation.byte_len.or_else(|| {
            let format = self.object_format(oop)?;
            if format.is_byte_format() {
                Some(self.slot_count(oop)? * 8 - format.padding_bytes())
            } else {
                None
            }
        })
    }

    pub fn set_byte_len(&mut self, oop: Oop, byte_len: usize) {
        if let Some(allocation) = self.allocation_mut(oop) {
            allocation.byte_len = Some(byte_len);
        }
    }

    pub fn bytes(&self, oop: Oop) -> Option<Vec<u8>> {
        let format = self.object_format(oop)?;
        if !format.is_byte_format() {
            return None;
        }
        let header = self.header(oop)?;
        let byte_len = self.byte_len(oop)?;
        let mut out = vec![0u8; byte_len];
        unsafe {
            let ptr = header.body_words() as *const u8;
            std::ptr::copy_nonoverlapping(ptr, out.as_mut_ptr(), byte_len);
        }
        Some(out)
    }

    pub fn write_byte(&mut self, oop: Oop, index: usize, value: u8) -> Option<()> {
        let format = self.object_format(oop)?;
        if !format.is_byte_format() {
            return None;
        }
        let header = self.header(oop)?;
        let byte_len = self.byte_len(oop)?;
        if index >= byte_len {
            return None;
        }
        unsafe {
            let ptr = header.body_words() as *mut u8;
            *ptr.add(index) = value;
        }
        Some(())
    }

    pub fn compiled_method_header(&self, oop: Oop) -> Option<MethodHeaderFields> {
        if self.object_format(oop)? != Format::CompiledMethod {
            return None;
        }
        MethodHeaderFields::decode(self.read_slot(oop, 0)?)
    }

    pub fn compiled_method_literal(&self, oop: Oop, index: usize) -> Option<Oop> {
        let header = self.compiled_method_header(oop)?;
        (index < header.num_literals as usize)
            .then(|| self.read_slot(oop, 1 + index))
            .flatten()
    }

    pub fn compiled_method_literals(&self, oop: Oop) -> Option<Vec<Oop>> {
        let header = self.compiled_method_header(oop)?;
        let mut out = Vec::with_capacity(header.num_literals as usize);
        for index in 0..header.num_literals as usize {
            out.push(self.compiled_method_literal(oop, index)?);
        }
        Some(out)
    }

    pub fn compiled_method_bytecodes(&self, oop: Oop) -> Option<Vec<u8>> {
        let method_header = self.compiled_method_header(oop)?;
        let header = self.header(oop)?;
        let byte_len = self.byte_len(oop)?;
        let mut out = vec![0u8; byte_len];
        unsafe {
            let ptr = header
                .body_words()
                .add(1 + method_header.num_literals as usize) as *const u8;
            std::ptr::copy_nonoverlapping(ptr, out.as_mut_ptr(), byte_len);
        }
        Some(out)
    }

    pub fn all_objects(&self) -> Vec<Oop> {
        self.allocations.iter().map(Allocation::oop).collect()
    }

    pub fn snapshots(&self) -> Vec<ObjectSnapshot> {
        self.allocations
            .iter()
            .map(|allocation| ObjectSnapshot {
                oop: allocation.oop(),
                header_raw: allocation.header_raw(),
                class_index: allocation.header_word().class_index(),
                format: allocation.header_word().format(),
                slot_count: allocation.slot_count(),
                generation: allocation.generation,
                byte_len: allocation.byte_len,
                payload_words: allocation.payload_words().to_vec(),
            })
            .collect()
    }

    pub fn object_header_word(&self, oop: Oop) -> Option<u64> {
        Some(unsafe { self.header(oop)?.header_word().raw() })
    }

    pub fn object_payload_words(&self, oop: Oop) -> Option<Vec<u64>> {
        let header = self.header(oop)?;
        let slot_count = unsafe { header.slot_count() };
        let mut out = Vec::with_capacity(slot_count);
        for index in 0..slot_count {
            out.push(unsafe { header.word(index) });
        }
        Some(out)
    }

    pub fn set_nursery_limit_bytes(&mut self, limit: usize) {
        self.nursery_limit_bytes = limit;
    }

    pub fn nursery_limit_bytes(&self) -> usize {
        self.nursery_limit_bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::class_table::{CLASS_INDEX_ARRAY, CLASS_INDEX_BYTE_ARRAY};

    use super::{GcKind, Generation, Heap};
    use crate::{object::Format, value::Oop};

    #[test]
    fn minor_gc_promotes_reachable_young_objects() {
        let mut heap = Heap::new();
        let young = heap.allocate_object(CLASS_INDEX_ARRAY, Format::VarPointers, 1);
        let old =
            heap.allocate_object_in(CLASS_INDEX_ARRAY, Format::VarPointers, 1, Generation::Old);
        heap.write_slot(old, 0, young);
        let result = heap.collect_garbage(&[old], GcKind::Minor);
        let new_young = result.relocated.get(&young.raw()).copied().unwrap();
        assert!(heap.is_old(new_young));
        assert_eq!(heap.read_slot(old, 0), Some(new_young));
        assert_eq!(result.promoted, 1);
    }

    #[test]
    fn full_gc_collects_unreachable_objects() {
        let mut heap = Heap::new();
        let live = heap.allocate_object(CLASS_INDEX_ARRAY, Format::VarPointers, 0);
        let dead = heap.allocate_object(CLASS_INDEX_ARRAY, Format::VarPointers, 0);
        let result = heap.collect_garbage(&[live], GcKind::Full);
        assert!(result.relocated.contains_key(&live.raw()));
        assert!(!result.relocated.contains_key(&dead.raw()));
        assert_eq!(heap.all_objects().len(), 1);
    }

    #[test]
    fn byte_objects_roundtrip() {
        let mut heap = Heap::new();
        let bytes = heap.allocate_bytes_in(CLASS_INDEX_BYTE_ARRAY, b"hello", Generation::Old);
        assert_eq!(heap.bytes(bytes).unwrap(), b"hello");
        assert_eq!(heap.byte_len(bytes), Some(5));
        assert!(Oop::nil().is_nil());
    }
}
