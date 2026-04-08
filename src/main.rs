use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use smallesttalk::{
    Format, Oop, Vm, compile_doit, compile_method_source, image, load_source,
    load_source_with_smalltalk_compiler, load_source_with_smalltalk_compiler_two_phase,
};

fn default_image_path() -> PathBuf {
    PathBuf::from("smallesttalk.img")
}

fn load_or_boot(path: &Path) -> Vm {
    if path.exists() {
        match image::load_vm(path) {
            Ok(vm) => return vm,
            Err(err) => eprintln!("warning: failed to load {}: {err}", path.display()),
        }
    }

    let vm = Vm::new();
    if let Err(err) = image::save_vm(&vm, path) {
        eprintln!(
            "warning: failed to save bootstrap image {}: {err}",
            path.display()
        );
    }
    vm
}

fn print_help() {
    println!("commands:");
    println!("  help                    show this help");
    println!("  stats                   print VM statistics");
    println!("  classes                 list classes");
    println!("  symbols                 print symbol count");
    println!("  gc [minor|full]         run garbage collection");
    println!("  class NAME [ivars...]   create a fixed-pointer class and global binding");
    println!("  method CLASS SOURCE     compile a method into CLASS");
    println!("  set NAME EXPR           evaluate EXPR and store it in a global");
    println!("  doit EXPR               evaluate a Smalltalk expression");
    println!("  source FILE             load source chunks from FILE with the Rust bootstrap compiler");
    println!("  selfsource FILE         load source chunks from FILE with the in-image Smalltalk compiler");
    println!("  selfsource2 FILE...     load source files with the in-image compiler in two phases");
    println!("  save [path]             save image");
    println!("  load [path]             load image");
    println!("  quit                    save and exit");
    println!("  any other line is treated as a Smalltalk expression");
}

fn print_stats(vm: &Vm) {
    println!(
        "classes={}, objects={}, symbols={}, dirty_cards={}",
        vm.class_table.len().saturating_sub(1),
        vm.heap.all_objects().len(),
        vm.symbols.len(),
        vm.heap.dirty_card_count()
    );
}

fn find_class_by_name(vm: &Vm, name: &str) -> Option<u32> {
    vm.class_table
        .iter()
        .find_map(|(index, info)| (info.name == name).then_some(index))
}

fn format_oop(vm: &Vm, value: Oop) -> String {
    if value.is_nil() {
        return "nil".to_string();
    }
    if value == vm.true_oop() {
        return "true".to_string();
    }
    if value == vm.false_oop() {
        return "false".to_string();
    }
    if let Some(int) = value.as_i64() {
        return int.to_string();
    }
    if let Ok(class_index) = vm.class_of(value) {
        if class_index == smallesttalk::class_table::CLASS_INDEX_SYMBOL {
            if let Ok(text) = vm.symbol_text(value) {
                return format!("#{text}");
            }
        }
        if class_index == smallesttalk::class_table::CLASS_INDEX_STRING {
            if let Ok(text) = vm.symbol_text(value) {
                return format!("'{text}'");
            }
        }
        if let Some(info) = vm.class_table.get(class_index) {
            return format!("<{} {:?}>", info.name, value);
        }
    }
    format!("{value:?}")
}

fn eval_doit(vm: &mut Vm, source: &str) {
    match compile_doit(vm, source) {
        Ok(method) => match vm.run_method(method, Oop::nil(), &[]) {
            Ok(result) => println!("{}", format_oop(vm, result)),
            Err(err) => println!("execution error: {err}"),
        },
        Err(err) => println!("compile error: {err}"),
    }
}

fn repl(mut vm: Vm, mut image_path: PathBuf) -> io::Result<()> {
    println!("smallesttalk REPL");
    print_help();

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "help" {
            print_help();
        } else if line == "stats" {
            print_stats(&vm);
        } else if line == "classes" {
            for (index, info) in vm.class_table.iter() {
                println!("{index:>3} {}", info.name);
            }
        } else if line == "symbols" {
            println!("{}", vm.symbols.len());
        } else if line == "gc" || line == "gc minor" {
            let result = vm.minor_gc();
            println!(
                "minor gc: survivors={}, collected={}, promoted={}",
                result.survivors, result.collected, result.promoted
            );
        } else if line == "gc full" {
            let result = vm.full_gc();
            println!(
                "full gc: survivors={}, collected={}, promoted={}",
                result.survivors, result.collected, result.promoted
            );
        } else if let Some(rest) = line.strip_prefix("save") {
            let path = rest.trim();
            if !path.is_empty() {
                image_path = PathBuf::from(path);
            }
            match image::save_vm(&vm, &image_path) {
                Ok(()) => println!("saved {}", image_path.display()),
                Err(err) => println!("save failed: {err}"),
            }
        } else if let Some(rest) = line.strip_prefix("load") {
            let path = rest.trim();
            let path = if path.is_empty() {
                image_path.clone()
            } else {
                PathBuf::from(path)
            };
            match image::load_vm(&path) {
                Ok(new_vm) => {
                    vm = new_vm;
                    image_path = path;
                    println!("loaded {}", image_path.display());
                }
                Err(err) => println!("load failed: {err}"),
            }
        } else if let Some(path) = line.strip_prefix("source ") {
            match fs::read_to_string(path.trim()) {
                Ok(source) => match load_source(&mut vm, &source) {
                    Ok(summary) => println!(
                        "loaded source: classes={}, methods={}, doits={}",
                        summary.classes, summary.methods, summary.doits
                    ),
                    Err(err) => println!("source load failed: {err}"),
                },
                Err(err) => println!("failed to read {}: {err}", path.trim()),
            }
        } else if let Some(path) = line.strip_prefix("selfsource ") {
            match fs::read_to_string(path.trim()) {
                Ok(source) => match load_source_with_smalltalk_compiler(&mut vm, &source) {
                    Ok(summary) => println!(
                        "self-loaded source: classes={}, methods={}, doits={}",
                        summary.classes, summary.methods, summary.doits
                    ),
                    Err(err) => println!("selfsource load failed: {err}"),
                },
                Err(err) => println!("failed to read {}: {err}", path.trim()),
            }
        } else if let Some(rest) = line.strip_prefix("selfsource2 ") {
            let paths = rest.split_whitespace().collect::<Vec<_>>();
            if paths.is_empty() {
                println!("usage: selfsource2 FILE...");
            } else {
                let mut combined = String::new();
                let mut failed = false;
                for (index, path) in paths.iter().enumerate() {
                    match fs::read_to_string(path) {
                        Ok(source) => {
                            if index > 0 {
                                combined.push_str("\n!\n");
                            }
                            combined.push_str(&source);
                        }
                        Err(err) => {
                            println!("failed to read {path}: {err}");
                            failed = true;
                            break;
                        }
                    }
                }
                if !failed {
                    match load_source_with_smalltalk_compiler_two_phase(&mut vm, &combined) {
                        Ok(summary) => println!(
                            "self-loaded source (two-phase): classes={}, methods={}, doits={}",
                            summary.classes, summary.methods, summary.doits
                        ),
                        Err(err) => println!("selfsource2 load failed: {err}"),
                    }
                }
            }
        } else if let Some(rest) = line.strip_prefix("class ") {
            let mut parts = rest.split_whitespace();
            if let Some(name) = parts.next() {
                let ivars = parts.map(str::to_string).collect::<Vec<_>>();
                match vm.new_class(
                    name,
                    Some(smallesttalk::class_table::CLASS_INDEX_BEHAVIOR),
                    Format::FixedPointers,
                    ivars.len(),
                ) {
                    Ok(class_index) => {
                        if let Err(err) = vm.set_instance_variables(class_index, ivars) {
                            println!("error: {err}");
                        } else if let Some(class_oop) = vm.class_table.class_oop(class_index) {
                            vm.set_global(name, class_oop);
                            println!("created class {name}");
                        }
                    }
                    Err(err) => println!("error: {err}"),
                }
            } else {
                println!("usage: class NAME [ivars...]");
            }
        } else if let Some(rest) = line.strip_prefix("method ") {
            let rest = rest.trim();
            if let Some((class_name, source)) = rest.split_once(' ') {
                match find_class_by_name(&vm, class_name) {
                    Some(class_index) => match compile_method_source(&mut vm, class_index, source) {
                        Ok(_) => println!("compiled {class_name}>>{source}"),
                        Err(err) => println!("error: {err}"),
                    },
                    None => println!("unknown class: {class_name}"),
                }
            } else {
                println!("usage: method CLASS SOURCE");
            }
        } else if let Some(rest) = line.strip_prefix("set ") {
            let rest = rest.trim();
            if let Some((name, expr)) = rest.split_once(' ') {
                match compile_doit(&mut vm, expr) {
                    Ok(method) => match vm.run_method(method, Oop::nil(), &[]) {
                        Ok(value) => {
                            vm.set_global(name, value);
                            println!("{name} := {}", format_oop(&vm, value));
                        }
                        Err(err) => println!("execution error: {err}"),
                    },
                    Err(err) => println!("compile error: {err}"),
                }
            } else {
                println!("usage: set NAME EXPR");
            }
        } else if let Some(expr) = line.strip_prefix("doit ") {
            eval_doit(&mut vm, expr.trim());
        } else if line == "quit" || line == "exit" {
            break;
        } else {
            eval_doit(&mut vm, line);
        }
    }

    if let Err(err) = image::save_vm(&vm, &image_path) {
        eprintln!("warning: failed to save {}: {err}", image_path.display());
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let image_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(default_image_path);
    let vm = load_or_boot(&image_path);
    print_stats(&vm);

    if let Err(err) = repl(vm, image_path) {
        eprintln!("repl error: {err}");
    }
}
