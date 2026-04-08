use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},

};

use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use smallesttalk::{
    Format, Oop, Vm, compile_doit, compile_method_source, image, load_source,
    live_browser::{BrowserLayout, LiveBrowser, apply_browser_view, send_message},
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

fn load_source_file_into_vm(vm: &mut Vm, path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let summary = load_source(vm, &source)
        .map_err(|err| format!("failed to load {}: {err}", path.display()))?;
    println!(
        "loaded source: classes={}, methods={}, doits={}",
        summary.classes, summary.methods, summary.doits
    );
    Ok(())
}

fn run_source_file(path: &Path) -> Result<(), String> {
    let path = path.to_path_buf();
    std::thread::Builder::new()
        .name("render-source".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let mut vm = Vm::new();
            load_source_file_into_vm(&mut vm, &path)?;
            Ok::<(), String>(())
        })
        .map_err(|err| format!("failed to spawn render thread: {err}"))?
        .join()
        .map_err(|_| "render thread panicked".to_string())?
}

fn small_int(value: i64) -> Result<Oop, String> {
    Oop::from_i64(value).ok_or_else(|| format!("SmallInteger out of range: {value}"))
}

fn build_live_browser_ui(vm: &mut Vm, layout: &BrowserLayout) -> Result<(Oop, Oop, u32), String> {
    load_source(vm, include_str!("../smalltalk/gui/Bootstrap.st"))
        .map_err(|err| format!("failed to load GUI bootstrap: {err}"))?;

    let world_class = vm.global_value("World").ok_or("World class missing")?;
    let browser_class = vm
        .global_value("BrowserWindow")
        .ok_or("BrowserWindow class missing")?;
    let rectangle_class = vm.global_value("Rectangle").ok_or("Rectangle class missing")?;

    let world = send_message(vm, world_class, "new", &[]).map_err(|err| err.to_string())?;
    send_message(
        vm,
        world,
        "initializeWidth:height:depth:",
        &[
            small_int(layout.width as i64)?,
            small_int(layout.height as i64)?,
            small_int(1)?,
        ],
    )
    .map_err(|err| err.to_string())?;

    let browser = send_message(vm, browser_class, "new", &[]).map_err(|err| err.to_string())?;
    send_message(vm, browser, "initialize", &[]).map_err(|err| err.to_string())?;
    send_message(
        vm,
        browser,
        "classPaneWidth:",
        &[small_int(layout.class_pane_width as i64)?],
    )
    .map_err(|err| err.to_string())?;
    send_message(
        vm,
        browser,
        "methodPaneHeight:",
        &[small_int(layout.method_pane_height as i64)?],
    )
    .map_err(|err| err.to_string())?;

    let rect = send_message(vm, rectangle_class, "new", &[]).map_err(|err| err.to_string())?;
    let (x, y, w, h) = layout.browser_bounds();
    send_message(
        vm,
        rect,
        "setX:y:width:height:",
        &[small_int(x)?, small_int(y)?, small_int(w)?, small_int(h)?],
    )
    .map_err(|err| err.to_string())?;
    send_message(vm, browser, "bounds:", &[rect]).map_err(|err| err.to_string())?;
    send_message(vm, world, "addSubview:", &[browser]).map_err(|err| err.to_string())?;

    let host_display = send_message(vm, world, "hostDisplay", &[]).map_err(|err| err.to_string())?;
    let handle = send_message(vm, host_display, "handle", &[]).map_err(|err| err.to_string())?;
    let handle = handle.as_i64().ok_or("HostDisplay handle missing")? as u32;
    Ok((world, browser, handle))
}

fn snapshot_to_argb(snapshot: &smallesttalk::interpreter::HostDisplaySnapshot) -> Vec<u32> {
    match snapshot.depth {
        1 => (0..(snapshot.width * snapshot.height))
            .map(|pixel_index| {
                let byte = snapshot.last_frame.get(pixel_index / 8).copied().unwrap_or(0);
                let bit = 1u8 << (7 - (pixel_index % 8));
                if (byte & bit) != 0 {
                    0xff000000
                } else {
                    0xffffffff
                }
            })
            .collect(),
        8 => snapshot
            .last_frame
            .iter()
            .map(|value| {
                let value = *value as u32;
                0xff000000 | (value << 16) | (value << 8) | value
            })
            .collect(),
        _ => vec![0xffffffff; snapshot.width * snapshot.height],
    }
}

fn render_live_browser_snapshot(output_path: &Path, paths: &[String]) -> Result<(), String> {
    let mut vm = Vm::new();
    let layout = BrowserLayout::default();
    for path in paths {
        load_source_file_into_vm(&mut vm, Path::new(path))?;
    }
    let (world, browser_window, handle) = build_live_browser_ui(&mut vm, &layout)?;
    let browser = LiveBrowser::new(&vm);
    let view = browser.view_data(&vm, &layout);
    apply_browser_view(&mut vm, browser_window, &view).map_err(|err| err.to_string())?;
    let render_selector = vm.intern_symbol("render");
    vm.send(world, render_selector, &[]).map_err(|err| err.to_string())?;
    let snapshot = vm
        .host_display_snapshot(handle)
        .ok_or("no host display snapshot available")?;
    smallesttalk::gui_snapshot::write_display_png(
        output_path,
        snapshot.width,
        snapshot.height,
        snapshot.depth,
        &snapshot.last_frame,
    )
    .map_err(|err| format!("failed to save {}: {err}", output_path.display()))?;
    println!("saved {}", output_path.display());
    Ok(())
}

fn run_live_browser(paths: &[String]) -> Result<(), String> {
    let mut vm = Vm::new();
    let layout = BrowserLayout::default();
    for path in paths {
        load_source_file_into_vm(&mut vm, Path::new(path))?;
    }
    let (world, browser_window, handle) = build_live_browser_ui(&mut vm, &layout)?;
    let mut browser = LiveBrowser::new(&vm);
    let mut window = Window::new(
        "Smallesttalk Live Browser",
        layout.width,
        layout.height,
        WindowOptions::default(),
    )
    .map_err(|err| format!("failed to open window: {err}"))?;
    let render_selector = vm.intern_symbol("render");
    let mut last_mouse_down = false;
    let mut dirty = true;
    while window.is_open() {
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            match key {
                Key::Escape | Key::Q => return Ok(()),
                Key::Up => {
                    browser.move_up(&vm);
                    dirty = true;
                }
                Key::Down => {
                    browser.move_down(&vm);
                    dirty = true;
                }
                Key::Left => {
                    browser.focus_left();
                    dirty = true;
                }
                Key::Right => {
                    browser.focus_right();
                    dirty = true;
                }
                Key::Tab => {
                    browser.toggle_focus();
                    dirty = true;
                }
                Key::R => {
                    browser.refresh(&vm);
                    dirty = true;
                }
                _ => {}
            }
        }

        let mouse_down = window.get_mouse_down(MouseButton::Left);
        if mouse_down && !last_mouse_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
                let mx = mx.max(0.0) as usize;
                let my = my.max(0.0) as usize;
                if let Some(row) = layout.class_hit_row(mx, my) {
                    browser.click_class_row(&vm, &layout, row);
                    dirty = true;
                } else if let Some(row) = layout.method_hit_row(mx, my) {
                    browser.click_method_row(&vm, &layout, row);
                    dirty = true;
                }
            }
        }
        last_mouse_down = mouse_down;

        if dirty {
            browser.refresh(&vm);
            let view = browser.view_data(&vm, &layout);
            apply_browser_view(&mut vm, browser_window, &view).map_err(|err| err.to_string())?;
            vm.send(world, render_selector, &[]).map_err(|err| err.to_string())?;
            let snapshot = vm
                .host_display_snapshot(handle)
                .ok_or("no host display snapshot available")?;
            let buffer = snapshot_to_argb(&snapshot);
            window.set_title(&view.title);
            window
                .update_with_buffer(&buffer, snapshot.width, snapshot.height)
                .map_err(|err| format!("failed to present browser window: {err}"))?;
            dirty = false;
        } else {
            window.update();
        }
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(first) = args.next() {
        if first == "render" {
            let Some(path) = args.next() else {
                eprintln!("usage: cargo run -- render FILE");
                std::process::exit(2);
            };
            if let Err(err) = run_source_file(Path::new(&path)) {
                eprintln!("render failed: {err}");
                std::process::exit(1);
            }
            return;
        }
        if first == "browsepng" {
            let Some(path) = args.next() else {
                eprintln!("usage: cargo run -- browsepng OUT.png [FILE ...]");
                std::process::exit(2);
            };
            let paths = args.collect::<Vec<_>>();
            if let Err(err) = render_live_browser_snapshot(Path::new(&path), &paths) {
                eprintln!("browsepng failed: {err}");
                std::process::exit(1);
            }
            return;
        }
        if first == "browse" {
            let paths = args.collect::<Vec<_>>();
            if let Err(err) = run_live_browser(&paths) {
                eprintln!("browse failed: {err}");
                std::process::exit(1);
            }
            return;
        }

        let image_path = PathBuf::from(first);
        let vm = load_or_boot(&image_path);
        print_stats(&vm);

        if let Err(err) = repl(vm, image_path) {
            eprintln!("repl error: {err}");
        }
        return;
    }

    let image_path = default_image_path();
    let vm = load_or_boot(&image_path);
    print_stats(&vm);

    if let Err(err) = repl(vm, image_path) {
        eprintln!("repl error: {err}");
    }
}
