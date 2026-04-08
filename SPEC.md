# Smalltalk VM Specification

**Version 0.1 — Interpreter-only, 64-bit**

This document specifies a minimal Smalltalk-80 virtual machine targeting modern
64-bit hardware, implemented in Rust. The goal is the simplest architecture that
still gives acceptable performance: roughly 50–100× slower than C, fast enough
for interactive use and as a foundation for a later JIT.

---

## 1. Design Principles

1. **Everything is an object, but not everything is on the heap.**
   SmallIntegers (and later Characters) are encoded directly in the pointer
   using a tag bit, avoiding allocation for the most common values.

2. **Uniform object layout.** Every heap object is a header word followed by
   zero or more slots. The header encodes enough metadata that the GC and
   interpreter never need to inspect the class object itself during hot paths.

3. **Stack bytecodes.** A zero-operand-stack machine is the simplest to compile
   to and interpret. All operations push/pop an implicit operand stack.

4. **Sends are the only abstraction.** The bytecode set is tiny because all
   control flow, arithmetic, and logic reduce to message sends. Performance
   comes from caching and special-casing, not from a large instruction set.

---

## 2. Value Representation (Oop)

Every Smalltalk value — local variable, instance variable, array element,
stack slot — is a 64-bit **Oop** (object-oriented pointer).

### 2.1 Tag Scheme

```
Bit 0 = 0  →  Heap pointer (8-byte aligned, so low 3 bits are free)
Bit 0 = 1  →  SmallInteger, value in bits [63:1] as signed 63-bit int
```

This gives us an integer range of roughly ±4.6 × 10¹⁸, which covers all
practical integer use without boxing.

**Special case:** The bit pattern `0x0000_0000_0000_0000` (null pointer)
represents `nil`. It is not a valid heap pointer and is not a SmallInteger.

### 2.2 Operations on Oops

| Operation       | Cost   | Notes                                    |
|-----------------|--------|------------------------------------------|
| Integer add     | 2 insn | Add raw words, check overflow, fix tag   |
| Integer compare | 1 insn | Direct signed compare (tag cancels out)  |
| Class check     | 1 insn | Test bit 0                               |
| Pointer deref   | 0 insn | Raw value is already the address         |

### 2.3 Future Extension

A 3-bit tag scheme could add immediate Characters (tag `010`) and immediate
Floats (tag `100`, using the upper 61 bits as a truncated double). This is a
backward-compatible change — only the `is_immediate` / `class_of` logic needs
updating. We defer this.

---

## 3. Heap Object Layout

Every heap object has the following layout in memory:

```
┌─────────────────────────┐  ← object pointer points here
│  Header word (64 bits)  │
├─────────────────────────┤
│  Slot 0 (64 bits / Oop) │
│  Slot 1                 │
│  ...                    │
│  Slot N-1               │
└─────────────────────────┘
```

For byte-indexable objects (Strings, ByteArrays), the slots hold packed bytes
rather than Oops. The header's format field distinguishes the two cases.

### 3.1 Header Word

```
 63        40 39    36 35        14 13   12 11          0
┌────────────┬────────┬────────────┬───────┬─────────────┐
│ class_index│ format │  id_hash   │  gc   │    size     │
│  (24 bits) │(4 bits)│ (22 bits)  │(2 bit)│  (12 bits)  │
└────────────┴────────┴────────────┴───────┴─────────────┘
```

**class_index** (24 bits): Index into the global class table (§4). Supports
up to ~16 million classes. Using an index rather than a raw class pointer
saves 5 bytes per object versus a full 64-bit pointer and simplifies GC
(the class table is a root, individual headers are not traced for class refs).

**format** (4 bits): Encodes what kind of data the object's slots contain.
This is the critical dispatch point for the GC (what to trace) and for
primitive operations (how to index into the object).

| Value | Name              | Description                                     |
|-------|-------------------|-------------------------------------------------|
| 0     | `Empty`           | Zero-size object (UndefinedObject, True, False)  |
| 1     | `FixedPointers`   | Fixed pointer slots only (most objects)          |
| 2     | `VarPointers`     | Variable-length Oop array (Array)                |
| 3     | `FixedAndVar`     | Fixed fields + variable Oop tail                 |
| 4     | `Weak`            | Weak pointer slots (WeakArray)                   |
| 6     | `Words`           | 64-bit raw word array (LargePositiveInteger)     |
| 8–11  | `Bytes8/16/24/32` | Byte array; low 2 bits = unused trailing bytes   |
| 12    | `CompiledMethod`  | Literal frame (Oops) + bytecodes (bytes)         |

The `Bytes` sub-formats (8–11) encode how many of the trailing bytes in the
last 64-bit word are padding. For example, a 7-byte ByteArray occupies 1 word
(8 bytes) with format `Bytes + 1` indicating 1 unused trailing byte.

**id_hash** (22 bits): Identity hash code, assigned at allocation from a
global counter or PRNG. Supports up to ~4 million unique hashes before
collision; sufficient for identity-based hash tables.

**gc** (2 bits): Reserved for GC mark/forwarding state. See §10.

**size** (12 bits): Number of 64-bit slots following the header. Range 0–4094
for inline sizes. The value **4095 (0xFFF)** is an overflow sentinel: the
true size is stored in the 64-bit word *preceding* the header (a separate
overflow word allocated at creation time). The vast majority of objects have
fewer than 4095 slots, so this case is rare.

### 3.2 Object Size in Memory

```
total_bytes = 8 (header) + size * 8
```

For overflow objects:
```
total_bytes = 8 (overflow word) + 8 (header) + size * 8
```

All objects are 8-byte aligned. Minimum object size is 8 bytes (header only,
format = Empty).

---

## 4. Class Table

The class table is a growable array mapping `class_index → class Oop`. Each
class is itself a heap object (format `FixedPointers`) with well-known instance
variable layout:

| Slot | Name                  | Type              |
|------|-----------------------|-------------------|
| 0    | superclass            | Oop (class or nil) |
| 1    | method_dictionary     | Oop (MethodDict)  |
| 2    | format_descriptor     | SmallInteger      |
| 3    | instance_variable_names | Oop (Array of Symbols) |
| 4    | name                  | Oop (Symbol)      |
| 5    | subclasses            | Oop (Array)       |

The `format_descriptor` encodes the instance format (from §3.1) and the
number of fixed instance variables. The interpreter uses this to allocate
correctly-shaped instances.

### 4.1 Well-Known Class Indices

Index 0 is reserved (no class). The bootstrap image pre-populates:

| Index | Class                |
|-------|----------------------|
| 1     | UndefinedObject      |
| 2     | True                 |
| 3     | False                |
| 4     | SmallInteger         |
| 5     | Array                |
| 6     | ByteArray            |
| 7     | String (= ByteString)|
| 8     | Symbol               |
| 9     | BlockClosure         |
| 10    | CompiledMethod       |
| 11    | MethodContext         |
| 12    | Association          |
| 13    | MethodDictionary     |
| 14    | Character            |
| 15    | Float                |
| 16    | LargePositiveInteger |
| 17    | Message              |
| 18    | Behavior / Class     |

The SmallInteger class index is special: it is never stored in any header. The
interpreter returns it synthetically when asked for the class of an immediate
integer Oop.

---

## 5. Special Objects Array

A single well-known root array holds the objects the interpreter needs fast
access to without method lookup:

| Index | Object       |
|-------|--------------|
| 0     | `nil`        |
| 1     | `true`       |
| 2     | `false`      |
| 3     | Smalltalk (system dictionary) |
| 4     | Symbol table |
| 5     | `doesNotUnderstand:` selector |
| 6     | `cannotReturn:` selector |
| 7     | The class table array itself |

---

## 6. Compiled Method Format

A CompiledMethod is a heap object with format `CompiledMethod` (12). Its slot
area is split into two regions: a **literal frame** of Oops, followed by
**bytecodes** packed as raw bytes.

```
┌─────────────────────────────┐
│  Header (format = 12)       │  size = total 64-bit words
├─────────────────────────────┤
│  Method header word         │  slot 0: encoded metadata
├─────────────────────────────┤
│  Literal 0                  │  slot 1
│  Literal 1                  │  slot 2
│  ...                        │
│  Literal N-1                │  slot N
├─────────────────────────────┤
│  Bytecodes (packed bytes)   │  remaining words
│  ...                        │
└─────────────────────────────┘
```

### 6.1 Method Header Word (slot 0)

Encoded as a SmallInteger (so the GC skips it during pointer tracing):

```
 62       56 55     48 47       32 31            0
┌──────────┬─────────┬────────────┬───────────────┐
│ num_args │num_temps│ num_lits   │    flags      │
│ (7 bits) │(8 bits) │ (16 bits)  │  (32 bits)    │
└──────────┴─────────┴────────────┴───────────────┘
(shifted left 1 and OR'd with tag bit 1 to make it a SmallInteger)
```

**flags** includes: primitive index (10 bits, 0 = no primitive), large frame
flag, and a "has non-local return" flag for the closure machinery.

### 6.2 Literal Frame

The literal frame contains:

- **Constant objects** referenced by `pushLiteral` bytecodes (numbers, strings,
  arrays, symbols).
- **Selector symbols** referenced by `send` bytecodes. Each send bytecode
  indexes a literal slot to get the selector.
- **Association objects** for accessing globals and class variables — the
  `pushLiteralVariable` bytecode reads the `value` slot of the Association.

The last literal is conventionally the **method's class association**, used
for super sends.

---

## 7. Bytecode Instruction Set

Stack-based, variable-length encoding. One-byte opcodes for the most common
operations, two or three bytes for less common ones. The interpreter's inner
loop is `loop { match *ip { ... } }`.

### 7.1 Encoding Conventions

- `ip` is the instruction pointer, a byte offset into the bytecodes section.
- Operands follow the opcode byte. Multi-byte operands are big-endian.
- `stack[sp]` is the top of the operand stack. `sp` grows upward.

### 7.2 Opcode Map

#### Single-Byte (0x00–0x7F): High-Frequency Short Forms

```
0x00–0x0F  pushInstVar   n        Push receiver's inst var n (0–15)
0x10–0x1F  pushTemp      n        Push temporary/argument n (0–15)
0x20–0x2F  pushLiteral   n        Push literal n (0–15)
0x30–0x3F  pushLitVar    n        Push value of literal variable n (0–15)

0x40–0x47  popStoreInstVar n      Pop & store into inst var n (0–7)
0x48–0x4F  popStoreTemp  n        Pop & store into temp n (0–7)

0x50       pushSelf
0x51       pushNil
0x52       pushTrue
0x53       pushFalse
0x54       push -1
0x55       push 0
0x56       push 1
0x57       push 2
0x58       dup
0x59       pop

0x60–0x6F  sendShort     lit, argc   Send; lit index = low 4 bits of (opcode - 0x60)
                                      argc encoded in the literal (selector arity)
0x70–0x7F  sendSpecial    n          Arithmetic / comparison special send (see §7.3)
```

#### Two-Byte (0x80–0xBF)

```
0x80 nn    pushInstVar   nn         Extended (0–255)
0x81 nn    pushTemp      nn
0x82 nn    pushLiteral   nn
0x83 nn    pushLitVar    nn
0x84 nn    popStoreInstVar nn
0x85 nn    popStoreTemp  nn
0x86 nn    send          nn         Literal index nn (selector arity from selector)
0x87 nn    superSend     nn         Like send but start lookup at superclass
0x88 nn    jumpForward   nn         ip += nn
0x89 nn    jumpBack      nn         ip -= nn
0x8A nn    jumpTrue      nn         Pop, jump forward nn if true
0x8B nn    jumpFalse     nn         Pop, jump forward nn if false
0x8C nn    pushNewArray  nn         Create Array of size nn from stack
0x8D nn    push SmallInt nn         Push literal byte as SmallInteger (0–255)
```

#### Three-Byte (0xC0–0xDF)

```
0xC0 hi lo   extendedSend       lit_index = hi, arg_count = lo
0xC1 hi lo   extendedSuperSend
0xC2 hi lo   jumpForwardLong    offset = (hi << 8) | lo
0xC3 hi lo   jumpBackLong
0xC4 hi lo   jumpTrueLong
0xC5 hi lo   jumpFalseLong
```

#### Closure and Return (0xE0–0xEF)

```
0xE0 args copied size_hi size_lo
              pushClosure    Create a BlockClosure.
                             args = num block args
                             copied = num copied values (popped from stack)
                             size = byte count of the block's bytecodes (follows inline)

0xE1       returnTop           Return stack top from method (non-local if in block)
0xE2       returnSelf          Return receiver from method
0xE3       returnNil           Return nil from method
0xE4       blockReturn         Return from block to caller (local return)
```

### 7.3 Special Sends

Opcodes `0x70–0x7F` are **special sends** for the 16 most common selectors.
The interpreter fast-paths SmallInteger receivers before falling back to a
normal send.

| Opcode | Selector    | SmallInt fast path?  |
|--------|-------------|----------------------|
| 0x70   | `+`         | Yes                  |
| 0x71   | `-`         | Yes                  |
| 0x72   | `*`         | Yes                  |
| 0x73   | `/`         | (divide → check 0)  |
| 0x74   | `<`         | Yes                  |
| 0x75   | `>`         | Yes                  |
| 0x76   | `<=`        | Yes                  |
| 0x77   | `>=`        | Yes                  |
| 0x78   | `=`         | Yes                  |
| 0x79   | `~=`        | Yes                  |
| 0x7A   | `bitAnd:`   | Yes                  |
| 0x7B   | `bitOr:`    | Yes                  |
| 0x7C   | `bitShift:` | Yes                  |
| 0x7D   | `@`         | No (creates Point)   |
| 0x7E   | `at:`       | No (array bounds check) |
| 0x7F   | `at:put:`   | No (array bounds check) |

The fast path for `+` on SmallIntegers, for example:

1. Check both receiver and argument have tag bit 1.
2. Add the raw Oop values. Because both have tag bit 1, the result has bit 1
   from the carry — subtract 1 to fix the tag.
3. Check for overflow (the carry out of bit 62).
4. If any check fails, fall through to normal send.

This avoids method lookup for the vast majority of arithmetic.

---

## 8. Execution Model

### 8.1 Stack Layout

Frames are allocated on a contiguous C-like stack, not as heap objects. This
avoids heap pressure for activation records. The stack is a `Vec<Oop>` (or a
raw `[Oop]` slab) with a frame pointer (`fp`) and stack pointer (`sp`).

```
Stack growth →

┌──────────────────────────────────────────────────────────┐
│ ... caller's operand stack ...                           │
├────────────┬──────────┬────────┬───────┬─────────────────┤
│ saved_fp   │ saved_ip │ method │ recvr │ arg0 arg1 ...   │
├────────────┴──────────┴────────┴───────┴─────────────────┤
│ temp0  temp1  ...  tempN                                 │
├──────────────────────────────────────────────────────────┤
│ operand stack (grows →)                                  │
└──────────────────────────────────────────────────────────┘
 ↑ fp points here                                    sp ↑
```

| Frame field | Offset from fp | Description                        |
|-------------|----------------|------------------------------------|
| saved_fp    | 0              | Caller's frame pointer             |
| saved_ip    | 1              | Return address (byte offset)       |
| method      | 2              | Oop of the CompiledMethod          |
| receiver    | 3              | Oop of `self`                      |
| arg 0       | 4              | First argument                     |
| ...         | 4 + n          | Remaining arguments                |
| temp 0      | 4 + argc       | First temporary                    |
| ...         | 4 + argc + t   | Remaining temporaries (init to nil)|

The operand stack begins immediately after the last temporary.

### 8.2 Context Reification

If user code accesses `thisContext`, the interpreter **materializes** the
current stack frame as a heap-allocated MethodContext object, copies the frame
data into it, and patches the stack to point at the heap context. This is the
slow path — most code never touches `thisContext`, so the common case pays no
cost.

### 8.3 The Interpreter Loop

Pseudocode for the core loop:

```
loop {
    let opcode = bytecodes[ip];
    ip += 1;
    match opcode {
        PUSH_INST_VAR_0..=PUSH_INST_VAR_15 => {
            let idx = (opcode - PUSH_INST_VAR_0) as usize;
            let receiver_obj = unsafe { ObjHeader::from_oop(receiver) };
            stack[sp] = receiver_obj.slot(idx);
            sp += 1;
        }
        SEND_SPECIAL_ADD => {
            let arg = stack[sp - 1];
            let rcvr = stack[sp - 2];
            if rcvr.is_small_int() && arg.is_small_int() {
                // fast path: integer add with overflow check
                match rcvr.as_i64().checked_add(arg.as_i64()) {
                    Some(result) => {
                        sp -= 1;
                        stack[sp - 1] = Oop::from_i64(result);
                        continue; // skip method lookup entirely
                    }
                    None => { /* overflow — fall through to send */ }
                }
            }
            // slow path: normal send of #+ selector
            perform_send(SELECTOR_PLUS, 1);
        }
        RETURN_TOP => {
            let result = stack[sp - 1];
            sp = fp;
            ip = stack[fp + 1].as_i64() as usize; // saved_ip
            fp = stack[fp + 0].as_i64() as usize;  // saved_fp
            stack[sp - 1] = result; // replace receiver-arg area with result
        }
        // ... etc
    }
}
```

---

## 9. Method Lookup and Caching

### 9.1 The Global Method Cache

A direct-mapped hash table keyed on `(class_index, selector_oop) → method_oop`.
No chaining — collisions simply evict.

```
CACHE_SIZE = 2048  (power of two)

fn cache_index(class_index: u32, selector: Oop) -> usize {
    let h = (class_index as u64) ^ selector.raw();
    (h as usize >> 2) & (CACHE_SIZE - 1)
}
```

On each send:

1. Compute `class_index` of the receiver (tag check → SmallInteger index, or
   read header).
2. Probe the cache. If `(class, selector)` matches → use the cached method.
3. On miss: walk the superclass chain, searching each class's MethodDictionary.
   Store the result in the cache.

Empirically, a 2048-entry cache achieves ~95% hit rate in typical Smalltalk.

### 9.2 Inline Caches (Optional, High Value)

Each send site in the bytecodes caches the *last class seen*. The bytecode is
patched after the first execution:

```
Before:  SEND lit_idx
After:   CACHED_SEND lit_idx cached_class_index
```

On re-execution, compare receiver's class_index to the cached value. If it
matches, skip the global cache entirely and jump straight to the method. If it
misses, fall back to the global cache and re-patch.

This is a **monomorphic inline cache** (one class per site). It captures the
fact that most send sites see only one receiver type. Polymorphic inline caches
(PICs) are a later optimization.

---

## 10. Block Closures

A `BlockClosure` is a heap object (class index 9, format `FixedPointers`):

| Slot | Name          | Description                                    |
|------|---------------|------------------------------------------------|
| 0    | outer_context | Oop of the enclosing MethodContext or closure   |
| 1    | start_ip      | SmallInteger: bytecode offset of block body    |
| 2    | num_args      | SmallInteger: number of block arguments         |
| 3    | method        | Oop of the enclosing CompiledMethod             |
| 4..  | copied_values | Captured variables from enclosing scope         |

### 10.1 Creating Closures

The `pushClosure` bytecode (0xE0):

1. Pops `copied` values from the stack.
2. Allocates a BlockClosure object.
3. Fills in `outer_context` (current frame / context), `start_ip`, `num_args`,
   `method`, and the copied values.
4. Pushes the new closure onto the stack.
5. Advances `ip` past the block's bytecodes (they'll be entered via `value`).

### 10.2 Activating Closures

Sending `value`, `value:`, `value:value:`, etc. to a BlockClosure:

1. Push a new frame whose `method` is the closure's `method` field and whose
   `receiver` is the closure's home receiver.
2. Set `ip` to the closure's `start_ip`.
3. Temps accessed by the block are either in the copied values (read from the
   closure object) or in the enclosing context (accessed via `outer_context`).

### 10.3 Non-Local Return

`returnTop` (0xE1) inside a block must return from the **enclosing method**,
not just the block. The interpreter:

1. Walks `outer_context` links to find the home MethodContext.
2. If the home context is still on the stack, unwinds to it and returns.
3. If the home context has already returned (the method exited), signals
   `cannotReturn:`.

---

## 11. Garbage Collection

### 11.1 Nursery: Semi-Space Copying

New objects are allocated in a **nursery** (young generation) using bump-pointer
allocation. Two equal-sized semi-spaces (e.g. 4 MB each); only one is active.

**Allocation:**
```
fn allocate(size_bytes: usize) -> *mut u8 {
    let ptr = self.free;
    self.free += size_bytes;
    if self.free > self.limit {
        trigger_minor_gc();
        // retry
    }
    ptr
}
```

One pointer increment per allocation — extremely fast.

**Collection:** Copy all live nursery objects to the other semi-space using
Cheney's algorithm. Objects that survive N collections are **tenured** into
the old generation. Roots are: the stack, the global method cache, the class
table, and the special objects array.

**Pointer scanning:** The header format field tells the GC exactly what to scan:
- `FixedPointers`, `VarPointers`, `FixedAndVar`, `Weak`: all slots are Oops
- `Bytes`, `Words`: no slots are Oops
- `CompiledMethod`: first `num_literals + 1` slots are Oops, rest is bytes
- Tag-bit check on each Oop: skip SmallIntegers, trace heap pointers

### 11.2 Old Generation: Mark-Sweep

The old generation is a simple free-list allocator with mark-sweep collection:

1. **Mark:** Trace from roots, setting gc bit 0 on each reachable object.
2. **Sweep:** Walk the old space linearly. Unmarked objects go on the free list.
   Clear all mark bits.

### 11.3 Write Barrier

When a pointer store targets an old-generation object with a nursery-generation
value, we must record it so the minor GC knows to trace into old space. A
simple **card table** works: divide old space into 512-byte cards, mark a card
dirty on any pointer store into it.

```
fn oop_store(target: ObjHeader, slot: usize, value: Oop) {
    target.set_slot(slot, value);
    if target.is_old() && value.is_young() {
        card_table.mark_dirty(target.address());
    }
}
```

### 11.4 GC Bits in the Header

| Bit 1 | Bit 0 | Meaning                              |
|-------|-------|--------------------------------------|
| 0     | 0     | White (unmarked / unreached)         |
| 0     | 1     | Grey (reached, not yet scanned)      |
| 1     | 0     | Black (reached and scanned)          |
| 1     | 1     | Forwarded (body is a forwarding ptr) |

During semi-space copy, `11` means "this object has been copied; the first slot
now contains the forwarding address in to-space."

---

## 12. Rust Implementation Notes

### 12.1 Module Structure

```
src/
  main.rs              Entry point, image loading, REPL
  value.rs             Oop: tagged pointer type, conversions, tag checks
  object.rs            ObjHeader: header accessors, slot read/write
  heap.rs              Semi-space nursery, old-space, allocation, GC
  class_table.rs       Class table: index ↔ class Oop mapping
  bytecode.rs          Opcode constants, disassembler
  interpreter.rs       Main loop, frame management, send dispatch
  method_cache.rs      Global method cache
  primitives.rs        Numbered primitives (I/O, arithmetic overflow, etc.)
  image.rs             Snapshot loading / saving
  bootstrap.rs         Create initial kernel classes and methods
```

### 12.2 Safety Boundary

The interpreter core will necessarily use `unsafe` Rust for:

- Raw pointer arithmetic into the heap (object slot access).
- Tagged-pointer bit manipulation.
- The operand stack (unchecked indexing for speed).

The goal is to confine `unsafe` to the `value`, `object`, and `heap` modules,
presenting a safe API surface to `interpreter.rs` wherever practical. The GC is
inherently unsafe but should be well-encapsulated.

### 12.3 Key Rust Types

```rust
/// The universal Smalltalk value — a tagged 64-bit word.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Oop(u64);

/// View into a heap object. Not owning — the heap owns the memory.
#[derive(Clone, Copy)]
pub struct ObjHeader(*mut u64);

/// The operand/frame stack. A flat Vec<Oop> with fp/sp indices.
pub struct VmStack {
    slots: Vec<Oop>,
    sp: usize,
    fp: usize,
}

/// Top-level VM state.
pub struct Vm {
    heap: Heap,
    stack: VmStack,
    ip: usize,
    method: Oop,
    receiver: Oop,
    class_table: Vec<Oop>,
    method_cache: MethodCache,
    special_objects: Vec<Oop>,
}
```

### 12.4 Performance Considerations

- The inner interpreter loop should be a single large `match` in a tight
  function, so the compiler can generate a jump table.
- Mark the function `#[inline(never)]` to give the compiler a clear optimization
  scope (LLVM optimizes large match arms well when they're in one function).
- Use `unreachable_unchecked` for impossible opcode values after validation.
- Profile-guided optimization (PGO) is highly effective for interpreter
  dispatch and is worth using for release builds.

---

## 13. Bootstrapping

The VM starts with no image. The bootstrap sequence:

1. **Create kernel classes.** Allocate class objects for the well-known classes
   (§4.1) directly in the heap. Set up their superclass chains (Object →
   Behavior → Class, etc.).

2. **Populate method dictionaries.** Compile a minimal set of methods from
   Smalltalk source (or a pre-compiled representation) into CompiledMethod
   objects. At minimum: `doesNotUnderstand:`, basic arithmetic, `value` for
   BlockClosure, `new` for Behavior.

3. **Create special objects.** Allocate `nil`, `true`, `false` and fill the
   special objects array.

4. **Save an image.** Snapshot the heap to disk in a portable format. All
   subsequent boots load from the image rather than re-bootstrapping.

The image format can be a simple linear dump: `[header][heap bytes][class table][special objects]`. Pointers in the heap are stored as byte offsets from heap start, relocated at load time.

---

## 14. What's Not Covered (Yet)

- **JIT compilation.** The bytecode set is designed to be JIT-friendly (stack
  → SSA conversion is well-understood), but this spec covers only the
  interpreter.
- **Processes and scheduling.** Smalltalk green threads / process scheduling.
  Can be layered on top: the interpreter yields at backward jumps and sends.
- **Primitives.** Numbered primitives for I/O, file access, sockets, FFI.
  These are method-specific and called when the method header's primitive
  index is nonzero.
- **Finalization and ephemerons.**
- **Image segments and out-of-image code loading.**
