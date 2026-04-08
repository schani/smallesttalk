Compiler := CoreCompiler new.
Parsed := Compiler parseDoItSource: '1 + 2'.
Unit := Compiler compileDoItSource: '1 + 2'.
!
