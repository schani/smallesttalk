# CoreCompiler Smalltalk sources

These files are the beginning of the in-image Smalltalk compiler, written in `.st` files.

Files:
- `AST.st` — AST node classes
- `Frontend.st` — early frontend skeleton
- `Compiler.st` — compiler facade plus growing Smalltalk bytecode backend
- `RealFrontend.st` — more real scanner/parser work in Smalltalk
- `CompilerBootstrap.st` — bootstrap manifest that includes the real source files; no duplicated compiler source

Current status:
- loadable into the image with `source smalltalk/compiler/CompilerBootstrap.st`
- defines the compiler object model in Smalltalk
- provides an in-image compiler facade (`CoreCompiler`) and AST classes
- includes a Smalltalk-side scanner/parser that can already parse simple doits and methods
- includes a Smalltalk-side bytecode backend that can compile and run simple doits and install simple methods
- can self-recompile compiler source layers directly from the original `.st` source files
- can single-phase self-recompile the core compiler layers from source files
- can compile the whole compiler source set from source files in a two-phase pass so the compiler compiles its own sources in-image before installing the results
- the broadest fully robust path is still the two-phase pass, but the earlier single-phase core path is now stable

Smoke tests in the REPL:

```text
> source smalltalk/compiler/CompilerBootstrap.st
> Compiler := CoreCompiler new. Unit := Compiler compileDoItSource: '1 + 2'. Unit ast class name
#CoreMethodNode
```

```text
> source smalltalk/compiler/CompilerBootstrap.st
> source examples/compiler-frontend-smoke.st
> Parsed class name
#CoreMethodNode
> Parsed temporaries size
1
```

```text
> source smalltalk/compiler/CompilerBootstrap.st
> Compiler := CoreCompiler new. Unit := Compiler compileDoItSource: '1 + 2'.
> Unit compiledMethod
<CompiledMethod ...>
```

```text
> source smalltalk/compiler/CompilerBootstrap.st
> selfsource smalltalk/compiler/AST.st
> selfsource smalltalk/compiler/Compiler.st
> selfsource smalltalk/compiler/RealFrontend.st
```

```text
> source smalltalk/compiler/CompilerBootstrap.st
> selfsource2 smalltalk/compiler/AST.st smalltalk/compiler/Frontend.st smalltalk/compiler/Compiler.st smalltalk/compiler/RealFrontend.st
```
