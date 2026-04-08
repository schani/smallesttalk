# CoreSmalltalk Bootstrap Plan

This document defines the bootstrap subset of Smalltalk we will implement first in Rust.
Its purpose is to get us to a self-hosting parser/compiler:

1. implement a Rust scanner/parser/compiler for a strict subset,
2. use that subset to write the real parser/compiler in Smalltalk,
3. compile that into the image,
4. use the in-image compiler for a source REPL.

---

## 1. Scope

The first compiler targets **method-source Smalltalk** and **doit/workspace expressions**.
It now also supports a practical bootstrap/file-in subset via chunk loading and standard
class-creation messages, but it still does **not** implement the full classic Smalltalk
file-in/chunk language, pragmas, floats, byte arrays, or metaclass-side compilation.

The first deliverable is a Rust pipeline:

- scanner
- parser
- AST
- bytecode compiler
- method/doit compilation entry points

---

## 2. CoreSmalltalk v1 syntax

### 2.1 Literals

Supported initially:

- SmallInteger literals: `0`, `1`, `42`
- String literals: `'hello'`
- Symbol literals:
  - `#foo`
  - `#at:put:`
  - `#+`
- Literal arrays:
  - `#(1 foo 'bar' #baz nil true false)`
- Pseudo-literals:
  - `nil`
  - `true`
  - `false`

Deferred:

- floats
- characters
- byte arrays `#[...]`
- dynamic arrays `{...}`
- radix/scaled numbers

### 2.2 Variables

Supported:

- method arguments
- method temporaries
- block arguments
- block temporaries
- instance variables
- globals
- pseudo-vars:
  - `self`
  - `super`
  - `thisContext`

### 2.3 Expressions

Supported:

- variable reference
- literals
- parenthesized expressions
- blocks
- unary sends
- binary sends
- keyword sends
- cascades
- assignment
- return
- statement sequences

### 2.4 Blocks

Supported:

- `[]`
- `[:x | x + 1]`
- `[:x :y | x + y]`
- `[:x | | y | y := x + 1. y]`

Block semantics required by the compiler:

- copied values / closure capture
- local returns via `blockReturn`
- non-local returns via `^`

### 2.5 Methods

Supported:

- unary pattern
- binary pattern
- keyword pattern
- optional temp declarations
- statement body

Examples:

```smalltalk
size
    ^ count

+ anObject
    ^ self add: anObject

at: index put: value
    | old |
    old := self at: index.
    ^ old
```

---

## 3. Explicitly deferred from Rust bootstrap compiler

These are expected in the **full** compiler, but not required in the initial Rust subset:

- class definition syntax
- pragmas (`<primitive: ...>`)
- chunk/file-in syntax
- floats / characters / byte arrays
- pool dictionaries
- class vars in parser syntax
- optimizer passes beyond basic bytecode selection

---

## 4. Semantic rules

### 4.1 Precedence

Must match real Smalltalk:

1. unary
2. binary, left-associative
3. keyword
4. cascade

### 4.2 Name resolution

Compiler lookup order:

1. block args / temps
2. enclosing copied vars
3. method args / temps
4. instance vars
5. pseudo-vars
6. globals

### 4.3 Control flow

No special syntax for:

- `ifTrue:`
- `ifFalse:`
- `whileTrue:`
- `to:do:`

These remain ordinary message sends.

---

## 5. AST shape

The Rust parser should target a clean AST that the Smalltalk implementation can mirror.

### 5.1 Method AST

- selector
- argument names
- temporary names
- body statements
- source span

### 5.2 Statement AST

- expression statement
- assignment
- return

### 5.3 Expression AST

- literal
- variable
- pseudo-var
- send
- cascade
- block
- parenthesized / grouped expression

### 5.4 Literal AST

- integer
- string
- symbol
- literal array
- nil / true / false

### 5.5 Block AST

- args
- temps
- statements

---

## 6. Compiler output requirements

The Rust compiler must emit VM bytecodes already defined in `SPEC.md`:

- variable pushes/stores
- literal pushes
- special pushes (`self`, `nil`, `true`, `false`, small ints)
- send / superSend / special sends
- jumps as needed
- closure creation
- returns

The compiler does not need advanced optimization initially.
A correct baseline compiler is more important than a clever one.

---

## 7. Bootstrap phases

### Phase A — Rust bootstrap compiler

Implement in Rust:

1. scanner
2. parser
3. AST
4. bytecode compiler
5. method/doit entry points

Deliverables:

- compile method source string → `CompiledMethod`
- compile doit source string → executable method

### Phase B — Self-hosted CoreSmalltalk compiler

Use the Rust compiler to compile into the image:

- `Scanner`
- `Parser`
- `AstNode` classes
- `Compiler`
- `BytecodeBuilder`
- error classes

This compiler only needs to compile CoreSmalltalk first.

### Phase C — Full compiler in Smalltalk

Extend in-image compiler to support:

- full method syntax
- full class definition syntax
- pragmas
- full file-in/chunk reader
- richer literals
- REPL/workspace support

Note: the Rust side already provides a usable bootstrap chunk loader plus a small core
library (`ifTrue:`, `ifFalse:`, `whileTrue:`, `to:do:`, `ifNil:` variants, etc.), so the
in-image compiler can now be bootstrapped from source rather than only from host commands.

### Phase D — Full REPL

- read source
- parse/compile in image
- execute in workspace context
- print result

---

## 8. Rust implementation plan

### 8.1 Modules

Add:

```text
src/compiler/
  mod.rs
  token.rs
  scanner.rs
  ast.rs
  parser.rs
  encoder.rs
```

Initial milestone:

- `token.rs`: token kinds + spans
- `scanner.rs`: source → tokens
- `ast.rs`: syntax tree types
- `parser.rs`: methods + expressions
- `encoder.rs`: bytecode compiler skeleton, then implementation

### 8.2 Public entry points

Planned API:

```rust
parse_method(source: &str) -> Result<MethodDef, ParseError>
parse_doit(source: &str) -> Result<Doit, ParseError>
compile_method_source(vm: &mut Vm, class_index: u32, source: &str) -> Result<Oop, CompileError>
compile_doit(vm: &mut Vm, source: &str) -> Result<Oop, CompileError>
```

### 8.3 Milestones

#### M1
- scanner
- token spans
- tests for literals, selectors, temps, blocks

#### M2
- expression parser
- method parser
- AST pretty-good enough for compiler work

#### M3
- bytecode encoder for literals, vars, sends, returns
- compile simple methods

#### M4
- closures
- cascades
- super sends
- globals / literal variable support

#### M5
- doit compilation
- hook into REPL

---

## 9. Acceptance criteria for CoreSmalltalk compiler

We consider the Rust bootstrap compiler complete when it can compile methods using:

- unary / binary / keyword selectors
- temporaries
- assignments
- returns
- message precedence
- blocks with args and temps
- cascades
- literal arrays / strings / symbols / integers
- globals and instance vars
- `self`, `super`, `thisContext`

And when that compiled subset is rich enough to implement the real compiler in-image.

Status note: this is now largely true for the Rust-hosted bootstrap pipeline. The main
remaining work is defining the compiler classes in CoreSmalltalk source, filing them into
the image, and switching the active REPL/compiler path over to the in-image compiler.

---

## 10. Immediate next work

The Rust bootstrap compiler skeleton is no longer the immediate task; that part exists.
The next bootstrap steps are:

1. define the in-image compiler classes in CoreSmalltalk source
2. file them into the image using the Rust bootstrap loader
3. compile/evaluate source through those in-image compiler objects
4. then retire the Rust compiler from the interactive path except as a bootstrap tool
