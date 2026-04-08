use std::fs;

use crate::{
    class_table::{CLASS_INDEX_BEHAVIOR, CLASS_INDEX_STRING},
    compile_doit, compile_method_source,
    compiler::CompileError,
    heap::Generation,
    parse_method, Oop, Vm, VmError,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceLoadSummary {
    pub classes: usize,
    pub methods: usize,
    pub doits: usize,
}

#[derive(Debug)]
pub enum SourceLoadError {
    InvalidChunkHeader(String),
    MissingChunkBody(&'static str),
    UnknownSuperclass(String),
    UnknownClass(String),
    Io(String),
    Compile(CompileError),
    Vm(VmError),
}

impl std::fmt::Display for SourceLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidChunkHeader(header) => write!(f, "invalid source chunk header: {header}"),
            Self::MissingChunkBody(kind) => write!(f, "missing body for {kind} chunk"),
            Self::UnknownSuperclass(name) => write!(f, "unknown superclass: {name}"),
            Self::UnknownClass(name) => write!(f, "unknown class: {name}"),
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Compile(err) => write!(f, "compile error: {err}"),
            Self::Vm(err) => write!(f, "vm error: {err}"),
        }
    }
}

impl std::error::Error for SourceLoadError {}

impl From<CompileError> for SourceLoadError {
    fn from(value: CompileError) -> Self {
        Self::Compile(value)
    }
}

impl From<VmError> for SourceLoadError {
    fn from(value: VmError) -> Self {
        Self::Vm(value)
    }
}

pub fn load_source(vm: &mut Vm, source: &str) -> Result<SourceLoadSummary, SourceLoadError> {
    load_source_with_mode(vm, source, LoadMode::Rust)
}

pub fn load_source_with_smalltalk_compiler(
    vm: &mut Vm,
    source: &str,
) -> Result<SourceLoadSummary, SourceLoadError> {
    load_source_with_mode(vm, source, LoadMode::SmalltalkSinglePhase)
}

pub fn load_source_with_smalltalk_compiler_two_phase(
    vm: &mut Vm,
    source: &str,
) -> Result<SourceLoadSummary, SourceLoadError> {
    load_source_with_mode(vm, source, LoadMode::SmalltalkTwoPhase)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadMode {
    Rust,
    SmalltalkSinglePhase,
    SmalltalkTwoPhase,
}

fn load_source_with_mode(
    vm: &mut Vm,
    source: &str,
    mode: LoadMode,
) -> Result<SourceLoadSummary, SourceLoadError> {
    let mut summary = SourceLoadSummary::default();
    let mut methods_for_class: Option<String> = None;
    let mut staged_methods: Vec<(u32, Oop, Oop)> = Vec::new();
    for chunk in split_chunks(source) {
        let trimmed_chunk = chunk.trim();
        if trimmed_chunk.is_empty() {
            methods_for_class = None;
            continue;
        }
        let Some(header_line) = chunk.lines().find(|line| !line.trim().is_empty()) else {
            continue;
        };
        let header_line = header_line.trim();
        if let Some(class_name) = methods_for_class.clone() {
            if !is_directive_header(header_line) && parse_method(trimmed_chunk).is_ok() {
                let class_index = find_class_by_name(vm, &class_name)
                    .ok_or_else(|| SourceLoadError::UnknownClass(class_name.clone()))?;
                compile_method_chunk(vm, mode, class_index, trimmed_chunk, &mut staged_methods)?;
                summary.methods += 1;
                continue;
            }
            methods_for_class = None;
        }
        let body = chunk_after_header(&chunk, header_line).trim();
        if let Some(class_name) = parse_methods_for_header(header_line) {
            methods_for_class = Some(class_name.to_string());
            continue;
        }
        if let Some(path) = header_line.strip_prefix("include ") {
            let included = fs::read_to_string(path.trim())
                .map_err(|err| SourceLoadError::Io(format!("{}: {err}", path.trim())))?;
            let nested = load_source_with_mode(vm, &included, mode)?;
            summary.classes += nested.classes;
            summary.methods += nested.methods;
            summary.doits += nested.doits;
        } else if let Some(rest) = header_line.strip_prefix("class ") {
            let mut parts = rest.split_whitespace();
            let name = parts
                .next()
                .ok_or_else(|| SourceLoadError::InvalidChunkHeader(header_line.to_string()))?;
            let remaining = parts.collect::<Vec<_>>();
            let (superclass_name, ivar_names) = if remaining.first() == Some(&":") {
                if remaining.len() < 2 {
                    return Err(SourceLoadError::InvalidChunkHeader(header_line.to_string()));
                }
                (Some(remaining[1]), remaining[2..].to_vec())
            } else {
                (None, remaining)
            };
            let ivars = ivar_names.into_iter().map(str::to_string).collect::<Vec<_>>();
            let superclass = match superclass_name {
                Some("nil") => None,
                Some(name) => Some(
                    find_class_by_name(vm, name)
                        .ok_or_else(|| SourceLoadError::UnknownSuperclass(name.to_string()))?,
                ),
                None => Some(CLASS_INDEX_BEHAVIOR),
            };
            let class_index = if let Some(existing) = find_class_by_name(vm, name) {
                existing
            } else {
                vm.new_class(name, superclass, crate::Format::FixedPointers, ivars.len())?
            };
            vm.set_instance_variables(class_index, ivars)?;
            let class_oop = vm
                .class_table
                .class_oop(class_index)
                .ok_or_else(|| SourceLoadError::UnknownClass(name.to_string()))?;
            vm.set_global(name, class_oop);
            summary.classes += 1;
        } else if let Some(rest) = header_line.strip_prefix("method ") {
            let class_name = rest.trim();
            if body.is_empty() {
                return Err(SourceLoadError::MissingChunkBody("method"));
            }
            let class_index = find_class_by_name(vm, class_name)
                .ok_or_else(|| SourceLoadError::UnknownClass(class_name.to_string()))?;
            compile_method_chunk(vm, mode, class_index, body, &mut staged_methods)?;
            summary.methods += 1;
        } else if let Some((class_name, _)) = header_line.split_once(">>") {
            let class_name = class_name.trim();
            if body.is_empty() {
                return Err(SourceLoadError::MissingChunkBody("method"));
            }
            let class_index = find_class_by_name(vm, class_name)
                .ok_or_else(|| SourceLoadError::UnknownClass(class_name.to_string()))?;
            compile_method_chunk(vm, mode, class_index, body, &mut staged_methods)?;
            summary.methods += 1;
        } else if header_line == "doit" {
            if body.is_empty() {
                return Err(SourceLoadError::MissingChunkBody("doit"));
            }
            run_doit_chunk(vm, mode, body)?;
            summary.doits += 1;
        } else {
            run_doit_chunk(vm, mode, trimmed_chunk)?;
            summary.doits += 1;
        }
    }
    if mode == LoadMode::SmalltalkTwoPhase {
        for (class_index, selector_oop, method) in staged_methods {
            let selector_text = vm.symbol_text(selector_oop)?;
            let selector = vm.intern_symbol(&selector_text);
            vm.add_method(class_index, selector, method)?;
        }
    }
    Ok(summary)
}

fn compile_method_chunk(
    vm: &mut Vm,
    mode: LoadMode,
    class_index: u32,
    source: &str,
    staged_methods: &mut Vec<(u32, Oop, Oop)>,
) -> Result<(), SourceLoadError> {
    match mode {
        LoadMode::Rust => {
            compile_method_source(vm, class_index, source)?;
        }
        LoadMode::SmalltalkSinglePhase => {
            compile_method_source_with_smalltalk_compiler(vm, class_index, source)?;
        }
        LoadMode::SmalltalkTwoPhase => {
            let (selector, method) = compile_method_source_with_smalltalk_compiler_without_install(
                vm,
                class_index,
                source,
            )?;
            staged_methods.push((class_index, selector, method));
        }
    }
    Ok(())
}

fn run_doit_chunk(vm: &mut Vm, mode: LoadMode, source: &str) -> Result<(), SourceLoadError> {
    match mode {
        LoadMode::Rust => {
            let method = compile_doit(vm, source)?;
            let _ = vm.run_method(method, Oop::nil(), &[])?;
        }
        LoadMode::SmalltalkSinglePhase | LoadMode::SmalltalkTwoPhase => {
            run_doit_with_smalltalk_compiler(vm, source)?;
        }
    }
    Ok(())
}

fn new_smalltalk_string(vm: &mut Vm, text: &str) -> Oop {
    vm.heap
        .allocate_bytes_in(CLASS_INDEX_STRING, text.as_bytes(), Generation::Old)
}

fn new_core_compiler(vm: &mut Vm) -> Result<Oop, SourceLoadError> {
    let compiler_class = vm
        .global_value("CoreCompiler")
        .ok_or_else(|| SourceLoadError::UnknownClass("CoreCompiler".to_string()))?;
    let new_selector = vm.intern_symbol("new");
    Ok(vm.send(compiler_class, new_selector, &[])? )
}

fn compile_method_source_with_smalltalk_compiler(
    vm: &mut Vm,
    class_index: u32,
    source: &str,
) -> Result<Oop, SourceLoadError> {
    let compiler = new_core_compiler(vm)?;
    let source_oop = new_smalltalk_string(vm, source);
    let class_oop = vm
        .class_table
        .class_oop(class_index)
        .ok_or(SourceLoadError::UnknownClass(format!("class index {class_index}")))?;
    let selector = vm.intern_symbol("compileAndInstallMethodSource:forClass:");
    Ok(vm.send(compiler, selector, &[source_oop, class_oop])?)
}

fn compile_method_source_with_smalltalk_compiler_without_install(
    vm: &mut Vm,
    class_index: u32,
    source: &str,
) -> Result<(Oop, Oop), SourceLoadError> {
    let compiler = new_core_compiler(vm)?;
    let source_oop = new_smalltalk_string(vm, source);
    let class_oop = vm
        .class_table
        .class_oop(class_index)
        .ok_or(SourceLoadError::UnknownClass(format!("class index {class_index}")))?;
    let compile_selector = vm.intern_symbol("compileMethodSourceWithoutInstall:forClass:");
    let unit = vm.send(compiler, compile_selector, &[source_oop, class_oop])?;
    let ast_selector = vm.intern_symbol("ast");
    let compiled_method_selector = vm.intern_symbol("compiledMethod");
    let selector_selector = vm.intern_symbol("selector");
    let ast = vm.send(unit, ast_selector, &[])?;
    let selector = vm.send(ast, selector_selector, &[])?;
    let method = vm.send(unit, compiled_method_selector, &[])?;
    Ok((selector, method))
}

fn run_doit_with_smalltalk_compiler(vm: &mut Vm, source: &str) -> Result<Oop, SourceLoadError> {
    let compiler = new_core_compiler(vm)?;
    let source_oop = new_smalltalk_string(vm, source);
    let compile_selector = vm.intern_symbol("compileDoItSource:");
    let unit = vm.send(compiler, compile_selector, &[source_oop])?;
    let compiled_method_selector = vm.intern_symbol("compiledMethod");
    let method = vm.send(unit, compiled_method_selector, &[])?;
    Ok(vm.run_method(method, Oop::nil(), &[])?)
}

fn split_chunks(source: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '!' {
            if chars.peek() == Some(&'!') {
                current.push('!');
                chars.next();
            } else {
                chunks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn parse_methods_for_header(header_line: &str) -> Option<&str> {
    let (class_name, rest) = header_line.split_once(" methodsFor:")?;
    if rest.trim_start().starts_with('\'') {
        Some(class_name.trim())
    } else {
        None
    }
}

fn is_directive_header(header_line: &str) -> bool {
    header_line == "doit"
        || header_line.starts_with("include ")
        || header_line.starts_with("class ")
        || header_line.starts_with("method ")
        || header_line.contains(">>")
        || parse_methods_for_header(header_line).is_some()
}

fn chunk_after_header<'a>(chunk: &'a str, header_line: &str) -> &'a str {
    let chunk = chunk.trim_start();
    &chunk[header_line.len()..]
}

fn find_class_by_name(vm: &Vm, name: &str) -> Option<u32> {
    vm.class_table
        .iter()
        .find_map(|(index, info)| (info.name == name).then_some(index))
}

#[cfg(test)]
mod tests {
    use super::{
        load_source, load_source_with_smalltalk_compiler,
        load_source_with_smalltalk_compiler_two_phase,
    };
    use crate::{Oop, Vm};

    #[test]
    fn loads_classes_methods_and_doits() {
        let mut vm = Vm::new();
        let summary = load_source(
            &mut vm,
            "class Point : Behavior x y\n!\nmethod Point\nx\n    ^ x\n!\nmethod Point\nx: value\n    x := value\n!\ndoit\nP := Point new.\nP x: 41.\n!\n",
        )
        .unwrap();
        assert_eq!(summary.classes, 1);
        assert_eq!(summary.methods, 2);
        assert_eq!(summary.doits, 1);
        let point = vm.global_value("P").unwrap();
        let selector = vm.intern_symbol("x");
        let result = vm.send(point, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(41));
    }

    #[test]
    fn class_chunk_defaults_to_behavior_superclass() {
        let mut vm = Vm::new();
        load_source(&mut vm, "class Widget left right\n!\n").unwrap();
        let class_oop = vm.global_value("Widget").unwrap();
        let class_index = vm.class_table.class_index_of_oop(class_oop).unwrap();
        let info = vm.class_table.get(class_index).unwrap();
        assert_eq!(info.superclass, Some(crate::class_table::CLASS_INDEX_BEHAVIOR));
        assert_eq!(info.instance_variables, vec!["left", "right"]);
    }

    #[test]
    fn doit_chunk_can_return_result() {
        let mut vm = Vm::new();
        let summary = load_source(&mut vm, "doit\nAnswer := 42.\n!\n").unwrap();
        assert_eq!(summary.doits, 1);
        assert_eq!(vm.global_value("Answer").and_then(Oop::as_i64), Some(42));
    }

    #[test]
    fn loads_smalltalkish_class_creation_and_method_headers() {
        let mut vm = Vm::new();
        let summary = load_source(
            &mut vm,
            "doit\nBehavior subclass: #Node instanceVariableNames: 'next'\n!\nNode >>\nnext\n    ^ next\n!\nNode >>\nnext: value\n    next := value\n!\ndoit\nN := Node new.\nN next: 5.\n!\n",
        )
        .unwrap();
        assert_eq!(summary.methods, 2);
        let node = vm.global_value("N").unwrap();
        let selector = vm.intern_symbol("next");
        let result = vm.send(node, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(5));
    }

    #[test]
    fn loads_methods_for_sections() {
        let mut vm = Vm::new();
        let summary = load_source(
            &mut vm,
            "doit\nBehavior subclass: #Point instanceVariableNames: 'x'\n!\nPoint methodsFor: 'accessing'\n!\nx\n    ^ x\n!\nx: value\n    x := value\n!\ndoit\nP := Point new. P x: 9\n!\n",
        )
        .unwrap();
        assert_eq!(summary.methods, 2);
        let point = vm.global_value("P").unwrap();
        let selector = vm.intern_symbol("x");
        let result = vm.send(point, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(9));
    }

    #[test]
    fn loads_implicit_doit_chunks() {
        let mut vm = Vm::new();
        let summary = load_source(
            &mut vm,
            "Behavior subclass: #ImplicitPoint instanceVariableNames: 'x'
!
ImplicitPoint >>
x
    ^ x
!
ImplicitPoint >>
x: value
    x := value
!
P := ImplicitPoint new. P x: 11
!
",
        )
        .unwrap();
        assert_eq!(summary.doits, 2);
        assert_eq!(summary.methods, 2);
        let point = vm.global_value("P").unwrap();
        let selector = vm.intern_symbol("x");
        let result = vm.send(point, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(11));
    }

    #[test]
    fn loads_implicit_doits_with_methods_for_sections() {
        let mut vm = Vm::new();
        let summary = load_source(
            &mut vm,
            "Behavior subclass: #SectionPoint instanceVariableNames: 'x'
!
SectionPoint methodsFor: 'accessing'
!
x
    ^ x
!
x: value
    x := value
!
P := SectionPoint new. P x: 21
!
",
        )
        .unwrap();
        assert_eq!(summary.doits, 2);
        assert_eq!(summary.methods, 2);
        let point = vm.global_value("P").unwrap();
        let selector = vm.intern_symbol("x");
        let result = vm.send(point, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(21));
    }

    #[test]
    fn chunk_parser_allows_escaped_bang() {
        let mut vm = Vm::new();
        let summary = load_source(&mut vm, "Bang := 'a!!b'\n!\n").unwrap();
        assert_eq!(summary.doits, 1);
        let bang = vm.global_value("Bang").unwrap();
        let text = vm.symbol_text(bang).unwrap();
        assert_eq!(text, "a!b");
    }

    #[test]
    fn loads_standard_smalltalk_class_definition_message() {
        let mut vm = Vm::new();
        let summary = load_source(
            &mut vm,
            "Behavior subclass: #StdPoint instanceVariableNames: 'x y' classVariableNames: '' poolDictionaries: '' category: 'Demo'\n!\nStdPoint >>\nx\n    ^ x\n!\nStdPoint >>\nx: value\n    x := value\n!\nP := StdPoint new yourself. P x: 17\n!\n",
        )
        .unwrap();
        assert_eq!(summary.methods, 2);
        let point = vm.global_value("P").unwrap();
        let selector = vm.intern_symbol("x");
        let result = vm.send(point, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(17));
    }

    #[test]
    fn loads_smalltalk_compiler_sources_from_st_file() {
        let mut vm = Vm::new();
        let summary = load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
        assert!(summary.methods >= 40);
        assert!(vm.global_value("CoreCompiler").is_some());
        let method = crate::compile_doit(&mut vm, "Compiler := CoreCompiler new. Unit := Compiler compileDoItSource: '1 + 2'. Unit ast class name").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(vm.symbol_text(result).unwrap(), "CoreMethodNode");
    }

    #[test]
    fn smalltalk_compiler_can_self_compile_ast_layer() {
        std::thread::Builder::new()
            .name("ast-self-compile".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let mut vm = Vm::new();
                let bootstrap = load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
                let ast = load_source_with_smalltalk_compiler(
                    &mut vm,
                    include_str!("../smalltalk/compiler/AST.st"),
                )
                .unwrap();
                assert!(bootstrap.methods >= 100);
                assert!(ast.methods >= 20);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn smalltalk_compiler_can_self_compile_core_in_single_phase() {
        std::thread::Builder::new()
            .name("single-phase-core-self-compile".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let mut vm = Vm::new();
                load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
                let ast = load_source_with_smalltalk_compiler(
                    &mut vm,
                    include_str!("../smalltalk/compiler/AST.st"),
                )
                .unwrap();
                let compiler = load_source_with_smalltalk_compiler(
                    &mut vm,
                    include_str!("../smalltalk/compiler/Compiler.st"),
                )
                .unwrap();
                let frontend = load_source_with_smalltalk_compiler(
                    &mut vm,
                    include_str!("../smalltalk/compiler/RealFrontend.st"),
                )
                .unwrap();
                assert!(ast.methods >= 20);
                assert!(compiler.methods >= 80);
                assert!(frontend.methods >= 40);
                let method = crate::compile_doit(
                    &mut vm,
                    "Compiler := CoreCompiler new. Unit := Compiler compileDoItSource: '1 + 2'. Unit compiledMethod",
                )
                .unwrap();
                let compiled = vm.run_method(method, Oop::nil(), &[]).unwrap();
                let result = vm.run_method(compiled, Oop::nil(), &[]).unwrap();
                assert_eq!(result.as_i64(), Some(3));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn smalltalk_compiler_can_compile_its_own_sources_in_two_phases() {
        std::thread::Builder::new()
            .name("two-phase-self-compile".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let mut vm = Vm::new();
                load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
                let combined = [
                    include_str!("../smalltalk/compiler/AST.st"),
                    include_str!("../smalltalk/compiler/Frontend.st"),
                    include_str!("../smalltalk/compiler/Compiler.st"),
                    include_str!("../smalltalk/compiler/RealFrontend.st"),
                ]
                .join("\n!\n");
                let summary = load_source_with_smalltalk_compiler_two_phase(&mut vm, &combined).unwrap();
                assert!(summary.methods >= 150);
                let method = crate::compile_doit(
                    &mut vm,
                    "Compiler := CoreCompiler new. Unit := Compiler compileDoItSource: '1 + 2'. Unit compiledMethod",
                )
                .unwrap();
                let compiled = vm.run_method(method, Oop::nil(), &[]).unwrap();
                let result = vm.run_method(compiled, Oop::nil(), &[]).unwrap();
                assert_eq!(result.as_i64(), Some(3));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn loads_real_frontend_and_parses_simple_doit() {
        let mut vm = Vm::new();
        load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
        let method = crate::compile_doit(
            &mut vm,
            "Parser := CoreParser new. Parsed := Parser parseDoItSource: '| x | x := 1. x + 2'. Parsed class name",
        )
        .unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(vm.symbol_text(result).unwrap(), "CoreMethodNode");
        let method = crate::compile_doit(
            &mut vm,
            "Parser := CoreParser new. Parsed := Parser parseDoItSource: '| x | x := 1. x + 2'. Parsed temporaries size",
        )
        .unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(1));
    }

    #[test]
    fn smalltalk_compiler_can_compile_and_run_simple_doit() {
        let mut vm = Vm::new();
        load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
        let method = crate::compile_doit(
            &mut vm,
            "Compiler := CoreCompiler new. Unit := Compiler compileDoItSource: '1 + 2'. Unit compiledMethod",
        )
        .unwrap();
        let compiled = vm.run_method(method, Oop::nil(), &[]).unwrap();
        let result = vm.run_method(compiled, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
    }

    #[test]
    fn smalltalk_compiler_resolves_compiler_class_globals() {
        let mut vm = Vm::new();
        load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
        let method = crate::compile_doit(
            &mut vm,
            "Compiler := CoreCompiler new. Unit := Compiler compileDoItSource: '| u | u := CoreCompiledUnit new. u class name'. Unit compiledMethod",
        )
        .unwrap();
        let compiled = vm.run_method(method, Oop::nil(), &[]).unwrap();
        let result = vm.run_method(compiled, Oop::nil(), &[]).unwrap();
        assert_eq!(vm.symbol_text(result).unwrap(), "CoreCompiledUnit");
    }

    #[test]
    fn smalltalk_compiler_can_compile_and_install_simple_method() {
        let mut vm = Vm::new();
        load_source(&mut vm, include_str!("../smalltalk/compiler/CompilerBootstrap.st")).unwrap();
        let point = vm
            .new_class(
                "TinyPoint",
                Some(crate::class_table::CLASS_INDEX_BEHAVIOR),
                crate::Format::FixedPointers,
                1,
            )
            .unwrap();
        vm.set_instance_variables(point, vec!["x".to_string()]).unwrap();
        vm.set_global("TinyPoint", vm.class_table.class_oop(point).unwrap());
        let method = crate::compile_doit(
            &mut vm,
            "Compiler := CoreCompiler new. Compiler compileAndInstallMethodSource: 'x ^ x' forClass: TinyPoint. Compiler compileAndInstallMethodSource: 'x: value x := value' forClass: TinyPoint",
        )
        .unwrap();
        let _ = vm.run_method(method, Oop::nil(), &[]).unwrap();
        let object = vm.new_instance(point, 0).unwrap();
        let set_selector = vm.intern_symbol("x:");
        let get_selector = vm.intern_symbol("x");
        let _ = vm.send(object, set_selector, &[Oop::from_i64(41).unwrap()]).unwrap();
        let result = vm.send(object, get_selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(41));
    }
}
