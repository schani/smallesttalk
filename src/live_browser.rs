use crate::{
    class_table::CLASS_INDEX_ARRAY,
    heap::Generation,
    object::Format,
    Oop, Vm, VmError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserFocus {
    Classes,
    Methods,
}

#[derive(Clone, Debug)]
pub struct BrowserLayout {
    pub width: usize,
    pub height: usize,
    pub margin: usize,
    pub title_bar_height: usize,
    pub line_height: usize,
    pub class_pane_width: usize,
    pub method_pane_height: usize,
}

impl Default for BrowserLayout {
    fn default() -> Self {
        Self {
            width: 256,
            height: 160,
            margin: 8,
            title_bar_height: 8,
            line_height: 8,
            class_pane_width: 96,
            method_pane_height: 56,
        }
    }
}

impl BrowserLayout {
    pub fn browser_bounds(&self) -> (i64, i64, i64, i64) {
        (
            self.margin as i64,
            self.margin as i64,
            (self.width.saturating_sub(self.margin * 2)) as i64,
            (self.height.saturating_sub(self.margin * 2)) as i64,
        )
    }

    pub fn content_origin(&self) -> (usize, usize) {
        (self.margin + 1, self.margin + self.title_bar_height + 1)
    }

    pub fn content_extent(&self) -> (usize, usize) {
        let (_, _, w, h) = self.browser_bounds();
        (
            (w.max(0) as usize).saturating_sub(2),
            (h.max(0) as usize).saturating_sub(self.title_bar_height + 2),
        )
    }

    pub fn classes_visible_rows(&self) -> usize {
        let (_, content_h) = self.content_extent();
        content_h.saturating_sub(self.line_height + 3) / self.line_height
    }

    pub fn methods_visible_rows(&self) -> usize {
        self.method_pane_height
            .saturating_sub(self.line_height + 3)
            / self.line_height
    }

    pub fn source_visible_rows(&self) -> usize {
        let (_, content_h) = self.content_extent();
        content_h
            .saturating_sub(self.method_pane_height)
            .saturating_sub(self.line_height + 2)
            / self.line_height
    }

    pub fn class_hit_row(&self, x: usize, y: usize) -> Option<usize> {
        let (content_x, content_y) = self.content_origin();
        let (_content_w, content_h) = self.content_extent();
        if x < content_x || y < content_y {
            return None;
        }
        let class_w = self.class_pane_width;
        if x >= content_x + class_w || y >= content_y + content_h {
            return None;
        }
        let text_top = content_y + self.line_height + 2;
        if y < text_top {
            return None;
        }
        Some((y - text_top) / self.line_height)
    }

    pub fn method_hit_row(&self, x: usize, y: usize) -> Option<usize> {
        let (content_x, content_y) = self.content_origin();
        let (content_w, _) = self.content_extent();
        let right_x = content_x + self.class_pane_width - 1;
        if x < right_x || x >= content_x + content_w || y < content_y {
            return None;
        }
        if y >= content_y + self.method_pane_height {
            return None;
        }
        let text_top = content_y + self.line_height + 2;
        if y < text_top {
            return None;
        }
        Some((y - text_top) / self.line_height)
    }
}

#[derive(Clone, Debug, Default)]
pub struct BrowserViewData {
    pub class_lines: Vec<String>,
    pub method_lines: Vec<String>,
    pub source_lines: Vec<String>,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct LiveBrowser {
    classes: Vec<u32>,
    selected_class: usize,
    selected_method: usize,
    focus: BrowserFocus,
}

impl LiveBrowser {
    pub fn new(vm: &Vm) -> Self {
        let mut browser = Self {
            classes: Vec::new(),
            selected_class: 0,
            selected_method: 0,
            focus: BrowserFocus::Classes,
        };
        browser.refresh(vm);
        if let Some(index) = browser
            .classes
            .iter()
            .position(|class_index| vm.class_table.get(*class_index).map(|info| info.name.as_str()) == Some("BrowserWindow"))
        {
            browser.selected_class = index;
            browser.selected_method = 0;
            browser.refresh(vm);
        }
        browser
    }

    pub fn refresh(&mut self, vm: &Vm) {
        self.classes = vm.class_table.iter().map(|(index, _)| index).collect();
        self.classes.sort_by_key(|index| vm.class_table.get(*index).map(|info| info.name.clone()));
        if self.classes.is_empty() {
            self.selected_class = 0;
            self.selected_method = 0;
            return;
        }
        self.selected_class = self.selected_class.min(self.classes.len() - 1);
        let method_count = self.current_method_names(vm).len();
        self.selected_method = self.selected_method.min(method_count.saturating_sub(1));
    }

    pub fn move_up(&mut self, vm: &Vm) {
        match self.focus {
            BrowserFocus::Classes => {
                if self.selected_class > 0 {
                    self.selected_class -= 1;
                    self.selected_method = 0;
                }
            }
            BrowserFocus::Methods => {
                if self.selected_method > 0 {
                    self.selected_method -= 1;
                }
            }
        }
        self.refresh(vm);
    }

    pub fn move_down(&mut self, vm: &Vm) {
        match self.focus {
            BrowserFocus::Classes => {
                if self.selected_class + 1 < self.classes.len() {
                    self.selected_class += 1;
                    self.selected_method = 0;
                }
            }
            BrowserFocus::Methods => {
                let method_count = self.current_method_names(vm).len();
                if self.selected_method + 1 < method_count {
                    self.selected_method += 1;
                }
            }
        }
        self.refresh(vm);
    }

    pub fn focus_left(&mut self) {
        self.focus = BrowserFocus::Classes;
    }

    pub fn focus_right(&mut self) {
        self.focus = BrowserFocus::Methods;
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            BrowserFocus::Classes => BrowserFocus::Methods,
            BrowserFocus::Methods => BrowserFocus::Classes,
        };
    }

    pub fn click_class_row(&mut self, vm: &Vm, layout: &BrowserLayout, row: usize) {
        let start = window_start(self.classes.len(), self.selected_class, layout.classes_visible_rows());
        let index = start + row;
        if index < self.classes.len() {
            self.focus = BrowserFocus::Classes;
            self.selected_class = index;
            self.selected_method = 0;
            self.refresh(vm);
        }
    }

    pub fn click_method_row(&mut self, vm: &Vm, layout: &BrowserLayout, row: usize) {
        let names = self.current_method_names(vm);
        let start = window_start(names.len(), self.selected_method, layout.methods_visible_rows());
        let index = start + row;
        if index < names.len() {
            self.focus = BrowserFocus::Methods;
            self.selected_method = index;
            self.refresh(vm);
        }
    }

    pub fn view_data(&self, vm: &Vm, layout: &BrowserLayout) -> BrowserViewData {
        let class_names = self.class_names(vm);
        let method_names = self.current_method_names(vm);
        let class_start = window_start(class_names.len(), self.selected_class, layout.classes_visible_rows());
        let method_start = window_start(method_names.len(), self.selected_method, layout.methods_visible_rows());

        let class_lines = class_names
            .iter()
            .enumerate()
            .skip(class_start)
            .take(layout.classes_visible_rows())
            .map(|(index, name)| truncate_text(&decorate_line(name, index == self.selected_class, self.focus == BrowserFocus::Classes), 28))
            .collect();
        let method_lines = method_names
            .iter()
            .enumerate()
            .skip(method_start)
            .take(layout.methods_visible_rows())
            .map(|(index, name)| truncate_text(&decorate_line(name, index == self.selected_method, self.focus == BrowserFocus::Methods), 36))
            .collect();

        let mut source_lines = self
            .source_lines(vm)
            .into_iter()
            .map(|line| truncate_text(&line, 24))
            .collect::<Vec<_>>();
        source_lines.truncate(layout.source_visible_rows().max(1));
        let title = match (self.current_class_name(vm), self.current_method_names(vm).get(self.selected_method)) {
            (Some(class_name), Some(selector)) if !method_names.is_empty() => {
                format!("BROWSER {} {}", class_name, selector)
            }
            (Some(class_name), _) => format!("BROWSER {}", class_name),
            _ => "BROWSER".to_string(),
        };

        BrowserViewData {
            class_lines,
            method_lines,
            source_lines,
            title,
        }
    }

    pub fn current_class_name(&self, vm: &Vm) -> Option<String> {
        let class_index = *self.classes.get(self.selected_class)?;
        Some(vm.class_table.get(class_index)?.name.clone())
    }

    fn class_names(&self, vm: &Vm) -> Vec<String> {
        self.classes
            .iter()
            .filter_map(|index| vm.class_table.get(*index).map(|info| info.name.clone()))
            .collect()
    }

    fn current_method_names(&self, vm: &Vm) -> Vec<String> {
        let Some(class_index) = self.classes.get(self.selected_class).copied() else {
            return Vec::new();
        };
        let Some(info) = vm.class_table.get(class_index) else {
            return Vec::new();
        };
        let mut names = info
            .methods
            .keys()
            .filter_map(|selector| vm.symbol_text(*selector).ok())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn selected_method_source(&self, vm: &Vm) -> Option<String> {
        let class_index = self.classes.get(self.selected_class).copied()?;
        let info = vm.class_table.get(class_index)?;
        let selector_name = self.current_method_names(vm).get(self.selected_method)?.clone();
        let selector = info
            .methods
            .keys()
            .copied()
            .find(|selector| vm.symbol_text(*selector).ok().as_deref() == Some(selector_name.as_str()))?;
        let method = info.methods.get(&selector).copied()?;
        vm.method_source(method).map(ToString::to_string)
    }

    fn source_lines(&self, vm: &Vm) -> Vec<String> {
        let Some(class_index) = self.classes.get(self.selected_class).copied() else {
            return vec!["NO CLASS".to_string()];
        };
        let Some(info) = vm.class_table.get(class_index) else {
            return vec!["NO CLASS".to_string()];
        };
        let superclass = info
            .superclass
            .and_then(|index| vm.class_table.get(index))
            .map(|info| info.name.clone())
            .unwrap_or_else(|| "nil".to_string());
        let ivars = if info.instance_variables.is_empty() {
            "NONE".to_string()
        } else {
            info.instance_variables.join(" ")
        };
        let method_names = self.current_method_names(vm);
        let selected_method = method_names
            .get(self.selected_method)
            .cloned()
            .unwrap_or_else(|| "NONE".to_string());
        let mut lines = vec![
            format!("CLASS {}", info.name),
            format!("SUPER {}", superclass),
            format!("IVARS {}", ivars),
            format!("METHODS {}", method_names.len()),
            String::new(),
            format!("METHOD {}", selected_method),
        ];
        if let Some(source) = self.selected_method_source(vm) {
            lines.extend(source.lines().map(|line| line.to_string()));
        } else {
            lines.push("SOURCE UNAVAILABLE".to_string());
        }
        lines.extend([
            "ARROWS NAVIGATE".to_string(),
            "TAB SWITCH PANE".to_string(),
            "CLICK TO SELECT".to_string(),
            "ESC QUIT".to_string(),
        ]);
        lines
    }
}

fn window_start(total: usize, selected: usize, visible: usize) -> usize {
    if total <= visible || visible == 0 {
        return 0;
    }
    let half = visible / 2;
    let mut start = selected.saturating_sub(half);
    let max_start = total - visible;
    if start > max_start {
        start = max_start;
    }
    start
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let truncated = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn decorate_line(text: &str, selected: bool, focused: bool) -> String {
    match (selected, focused) {
        (true, true) => format!("> {text}"),
        (true, false) => format!("* {text}"),
        (false, _) => text.to_string(),
    }
}

pub fn apply_browser_view(
    vm: &mut Vm,
    browser_window: Oop,
    data: &BrowserViewData,
) -> Result<(), VmError> {
    let class_list = make_string_array(vm, &data.class_lines);
    let method_list = make_string_array(vm, &data.method_lines);
    let source_lines = make_string_array(vm, &data.source_lines);
    send_message(vm, browser_window, "classList:", &[class_list])?;
    send_message(vm, browser_window, "methodList:", &[method_list])?;
    send_message(vm, browser_window, "sourceLines:", &[source_lines])?;
    let title = make_string(vm, &data.title);
    send_message(vm, browser_window, "title:", &[title])?;
    Ok(())
}

pub fn make_string(vm: &mut Vm, text: &str) -> Oop {
    vm.heap
        .allocate_bytes_in(crate::class_table::CLASS_INDEX_STRING, text.as_bytes(), Generation::Old)
}

pub fn make_string_array(vm: &mut Vm, strings: &[String]) -> Oop {
    let array = vm
        .heap
        .allocate_object_in(CLASS_INDEX_ARRAY, Format::VarPointers, strings.len(), Generation::Old);
    for (index, string) in strings.iter().enumerate() {
        let value = make_string(vm, string);
        vm.heap.write_slot(array, index, value);
    }
    array
}

pub fn send_message(vm: &mut Vm, receiver: Oop, selector: &str, args: &[Oop]) -> Result<Oop, VmError> {
    let selector = vm.intern_symbol(selector);
    vm.send(receiver, selector, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_source, Vm};

    #[test]
    fn live_browser_lists_real_classes() {
        let mut vm = Vm::new();
        load_source(&mut vm, include_str!("../smalltalk/gui/Bootstrap.st")).unwrap();
        let browser = LiveBrowser::new(&vm);
        let data = browser.view_data(&vm, &BrowserLayout::default());
        assert!(data.class_lines.iter().any(|line| line.contains("BrowserWindow")));
    }

    #[test]
    fn live_browser_lists_methods_for_selected_class() {
        let mut vm = Vm::new();
        load_source(&mut vm, include_str!("../smalltalk/gui/Bootstrap.st")).unwrap();
        let mut browser = LiveBrowser::new(&vm);
        while browser.current_class_name(&vm).as_deref() != Some("BrowserWindow") {
            browser.move_down(&vm);
        }
        let data = browser.view_data(&vm, &BrowserLayout::default());
        assert!(!data.method_lines.is_empty());
        assert!(data.title.contains("BrowserWindow"));
        assert!(data.source_lines.iter().any(|line| line.contains("classList")));
    }
}
