use std::collections::{HashMap, HashSet};

use crate::{
    bytecode::{
        BLOCK_RETURN, DUP, POP, POP_STORE_INST_VAR_BASE, POP_STORE_INST_VAR_EXT,
        POP_STORE_TEMP_BASE, POP_STORE_TEMP_EXT, PUSH_CLOSURE, PUSH_FALSE, PUSH_INST_VAR_BASE,
        PUSH_INST_VAR_EXT, PUSH_LITERAL_BASE, PUSH_LITERAL_EXT, PUSH_LIT_VAR_BASE,
        PUSH_LIT_VAR_EXT, PUSH_MINUS_ONE, PUSH_NIL, PUSH_ONE, PUSH_SELF, PUSH_SMALL_INT_EXT,
        PUSH_TEMP_BASE, PUSH_TEMP_EXT, PUSH_TRUE, PUSH_TWO, PUSH_ZERO, RETURN_SELF, RETURN_TOP,
        SEND_EXT, SEND_SHORT_BASE, SEND_SPECIAL_BASE, SUPER_SEND_EXT,
    },
    class_table::CLASS_INDEX_STRING,
    compiler::{
        ast::{Block, CascadeMessage, Expression, Literal, MethodDef, PseudoVar, Statement},
        parse_doit, parse_method,
        parser::ParseError,
    },
    heap::Generation,
    object::MethodHeaderFields,
    value::Oop,
    Vm,
};

#[derive(Debug)]
pub enum CompileError {
    Parse(ParseError),
    Unsupported(&'static str),
    InvalidTempIndex(usize),
    InvalidInstVarIndex(usize),
    InvalidLiteralIndex(usize),
    InvalidSelectorLiteralIndex(usize),
    InvalidClosureLiteralIndex(usize),
    InvalidCascade,
    AssignmentToCaptured(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "parse error: {err}"),
            Self::Unsupported(msg) => write!(f, "unsupported construct: {msg}"),
            Self::InvalidTempIndex(index) => write!(f, "temp index out of range: {index}"),
            Self::InvalidInstVarIndex(index) => write!(f, "instance variable index out of range: {index}"),
            Self::InvalidLiteralIndex(index) => write!(f, "literal index out of range: {index}"),
            Self::InvalidSelectorLiteralIndex(index) => {
                write!(f, "selector literal index out of range: {index}")
            }
            Self::InvalidClosureLiteralIndex(index) => {
                write!(f, "closure literal index out of range: {index}")
            }
            Self::InvalidCascade => write!(f, "invalid cascade receiver"),
            Self::AssignmentToCaptured(name) => {
                write!(f, "assignment to captured variable is not supported yet: {name}")
            }
        }
    }
}

impl std::error::Error for CompileError {}

impl From<ParseError> for CompileError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

pub fn compile_method_source(
    vm: &mut Vm,
    class_index: u32,
    source: &str,
) -> Result<Oop, CompileError> {
    let method = parse_method(source)?;
    let selector = vm.intern_symbol(&method.pattern.selector);
    let compiled_method = UnitCompiler::for_method(vm, class_index, &method).compile()?;
    vm.add_method(class_index, selector, compiled_method)
        .map_err(|_| CompileError::Unsupported("failed to install compiled method"))?;
    vm.record_method_source(compiled_method, source);
    Ok(compiled_method)
}

pub fn compile_doit(vm: &mut Vm, source: &str) -> Result<Oop, CompileError> {
    let synthetic = parse_doit(source)?;
    UnitCompiler::for_method(vm, crate::class_table::CLASS_INDEX_UNDEFINED_OBJECT, &synthetic)
        .compile()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivationKind {
    Method,
    Block,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ResolvedName {
    Local(usize),
    Captured(usize),
    InstVar(usize),
    GlobalAssociation(Oop),
    GlobalValue(Oop),
}

struct UnitCompiler<'a> {
    vm: &'a mut Vm,
    class_index: u32,
    statements: &'a [Statement],
    literals: Vec<Oop>,
    bytecodes: Vec<u8>,
    locals: HashMap<String, usize>,
    captured_names: HashMap<String, usize>,
    boxed_local_names: HashSet<String>,
    total_arg_count: usize,
    explicit_temp_count: usize,
    hidden_temp_count: usize,
    activation: ActivationKind,
}

impl<'a> UnitCompiler<'a> {
    fn for_method(vm: &'a mut Vm, class_index: u32, method: &'a MethodDef) -> Self {
        let mut locals = HashMap::new();
        for (index, arg) in method.pattern.arguments.iter().enumerate() {
            locals.insert(arg.clone(), index);
        }
        for (temp_index, temp) in method.temps.iter().enumerate() {
            locals.insert(temp.clone(), method.pattern.arguments.len() + temp_index);
        }
        let boxed_local_names = collect_boxed_local_names(
            &method.statements,
            &method
                .pattern
                .arguments
                .iter()
                .chain(method.temps.iter())
                .cloned()
                .collect::<Vec<_>>(),
            &HashSet::new(),
        );
        let hidden_temp_count = usize::from(!boxed_local_names.is_empty());
        Self {
            vm,
            class_index,
            statements: &method.statements,
            literals: Vec::new(),
            bytecodes: Vec::new(),
            locals,
            captured_names: HashMap::new(),
            boxed_local_names,
            total_arg_count: method.pattern.arguments.len(),
            explicit_temp_count: method.temps.len(),
            hidden_temp_count,
            activation: ActivationKind::Method,
        }
    }

    fn for_block(
        vm: &'a mut Vm,
        class_index: u32,
        block: &'a Block,
        captured_names: &[String],
    ) -> Self {
        let mut locals = HashMap::new();
        let mut captured_map = HashMap::new();
        for (index, name) in captured_names.iter().enumerate() {
            locals.insert(name.clone(), index);
            captured_map.insert(name.clone(), index);
        }
        let arg_start = captured_names.len();
        for (index, arg) in block.args.iter().enumerate() {
            locals.insert(arg.clone(), arg_start + index);
        }
        let temp_start = arg_start + block.args.len();
        for (index, temp) in block.temps.iter().enumerate() {
            locals.insert(temp.clone(), temp_start + index);
        }
        let shadowed = captured_map.keys().cloned().collect::<HashSet<_>>();
        let boxed_local_names = collect_boxed_local_names(
            &block.statements,
            &block
                .args
                .iter()
                .chain(block.temps.iter())
                .cloned()
                .collect::<Vec<_>>(),
            &shadowed,
        );
        let hidden_temp_count = usize::from(!boxed_local_names.is_empty());
        Self {
            vm,
            class_index,
            statements: &block.statements,
            literals: Vec::new(),
            bytecodes: Vec::new(),
            locals,
            captured_names: captured_map,
            boxed_local_names,
            total_arg_count: captured_names.len() + block.args.len(),
            explicit_temp_count: block.temps.len(),
            hidden_temp_count,
            activation: ActivationKind::Block,
        }
    }

    fn compile(mut self) -> Result<Oop, CompileError> {
        self.emit_boxing_prologue()?;
        if self.statements.is_empty() {
            match self.activation {
                ActivationKind::Method => self.bytecodes.push(RETURN_SELF),
                ActivationKind::Block => {
                    self.bytecodes.push(PUSH_NIL);
                    self.bytecodes.push(BLOCK_RETURN);
                }
            }
        } else {
            for (index, statement) in self.statements.iter().enumerate() {
                let is_last = index + 1 == self.statements.len();
                self.compile_statement(statement, is_last)?;
            }
        }

        let header = MethodHeaderFields {
            num_args: self.total_arg_count as u8,
            num_temps: (self.explicit_temp_count + self.hidden_temp_count) as u8,
            num_literals: self.literals.len() as u16,
            flags: 0,
        };
        Ok(self.vm.compiled_method(header, &self.literals, &self.bytecodes))
    }

    fn scratch_temp_index(&self) -> Option<usize> {
        (self.hidden_temp_count > 0).then_some(self.total_arg_count + self.explicit_temp_count)
    }

    fn is_boxed_local_name(&self, name: &str) -> bool {
        self.boxed_local_names.contains(name)
    }

    fn emit_boxing_prologue(&mut self) -> Result<(), CompileError> {
        if self.boxed_local_names.is_empty() {
            return Ok(());
        }
        let scratch = self.scratch_temp_index().expect("boxed locals require scratch temp");
        let mut boxed = self
            .boxed_local_names
            .iter()
            .filter_map(|name| self.locals.get(name).copied().map(|index| (name.clone(), index)))
            .collect::<Vec<_>>();
        boxed.sort_by_key(|(_, index)| *index);
        for (_name, index) in boxed {
            self.emit_push_temp(index)?;
            self.emit_pop_store_temp(scratch)?;
            let association_class = self
                .vm
                .class_table
                .class_oop(crate::class_table::CLASS_INDEX_ASSOCIATION)
                .ok_or(CompileError::Unsupported("missing Association class"))?;
            let class_literal = self.add_literal(association_class);
            self.emit_push_literal(class_literal)?;
            self.emit_selector_send("new", false)?;
            self.bytecodes.push(DUP);
            self.emit_pop_store_temp(index)?;
            self.emit_push_temp(scratch)?;
            self.emit_selector_send("value:", false)?;
            self.bytecodes.push(POP);
        }
        Ok(())
    }

    fn compile_statement(&mut self, statement: &Statement, is_last: bool) -> Result<(), CompileError> {
        match statement {
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                if is_last {
                    self.emit_implicit_return();
                } else {
                    self.bytecodes.push(POP);
                }
            }
            Statement::Assignment { name, value, .. } => {
                let resolved = self.resolve_name(name);
                match resolved {
                    ResolvedName::Captured(index) => {
                        self.emit_push_temp(index)?;
                        self.compile_expression(value)?;
                        self.emit_selector_send("value:", false)?;
                        if is_last {
                            self.emit_implicit_return();
                        } else {
                            self.bytecodes.push(POP);
                        }
                    }
                    ResolvedName::GlobalAssociation(association) => {
                        let association_index = self.add_literal(association);
                        self.emit_push_literal(association_index)?;
                        self.compile_expression(value)?;
                        self.emit_selector_send("value:", false)?;
                        if is_last {
                            self.emit_implicit_return();
                        } else {
                            self.bytecodes.push(POP);
                        }
                    }
                    ResolvedName::GlobalValue(_) => {
                        return Err(CompileError::Unsupported("assignment to read-only global"));
                    }
                    ResolvedName::Local(index) => {
                        if self.is_boxed_local_name(name) {
                            self.emit_push_temp(index)?;
                            self.compile_expression(value)?;
                            self.emit_selector_send("value:", false)?;
                            if is_last {
                                self.emit_implicit_return();
                            } else {
                                self.bytecodes.push(POP);
                            }
                        } else {
                            self.compile_expression(value)?;
                            if is_last {
                                self.bytecodes.push(DUP);
                            }
                            self.emit_pop_store_temp(index)?;
                            if is_last {
                                self.emit_implicit_return();
                            }
                        }
                    }
                    ResolvedName::InstVar(index) => {
                        self.compile_expression(value)?;
                        if is_last {
                            self.bytecodes.push(DUP);
                        }
                        self.emit_pop_store_inst_var(index)?;
                        if is_last {
                            self.emit_implicit_return();
                        }
                    }
                }
            }
            Statement::Return { value, .. } => {
                self.compile_expression(value)?;
                self.bytecodes.push(RETURN_TOP);
            }
        }
        Ok(())
    }

    fn emit_implicit_return(&mut self) {
        self.bytecodes.push(match self.activation {
            ActivationKind::Method => RETURN_TOP,
            ActivationKind::Block => BLOCK_RETURN,
        });
    }

    fn compile_expression(&mut self, expr: &Expression) -> Result<(), CompileError> {
        match expr {
            Expression::Literal { value, .. } => self.emit_literal(value),
            Expression::Variable { name, .. } => match self.resolve_name(name) {
                ResolvedName::Local(index) => {
                    self.emit_push_temp(index)?;
                    if self.is_boxed_local_name(name) {
                        self.emit_selector_send("value", false)
                    } else {
                        Ok(())
                    }
                }
                ResolvedName::Captured(index) => {
                    self.emit_push_temp(index)?;
                    self.emit_selector_send("value", false)
                }
                ResolvedName::InstVar(index) => self.emit_push_inst_var(index),
                ResolvedName::GlobalAssociation(association) => {
                    let literal_index = self.add_literal(association);
                    self.emit_push_lit_var(literal_index)
                }
                ResolvedName::GlobalValue(value) => {
                    let literal_index = self.add_literal(value);
                    self.emit_push_literal(literal_index)
                }
            },
            Expression::PseudoVar { value, .. } => match value {
                PseudoVar::Self_ => {
                    self.bytecodes.push(PUSH_SELF);
                    Ok(())
                }
                PseudoVar::ThisContext => {
                    self.bytecodes.push(PUSH_SELF);
                    self.emit_selector_send("thisContext", false)
                }
                PseudoVar::Super => Err(CompileError::Unsupported("super as bare expression")),
            },
            Expression::Send {
                receiver,
                selector,
                arguments,
                ..
            } => self.compile_send(receiver, selector, arguments),
            Expression::Cascade { head, messages, .. } => self.compile_cascade(head, messages),
            Expression::Block(block) => self.compile_block(block),
        }
    }

    fn compile_send(
        &mut self,
        receiver: &Expression,
        selector: &str,
        arguments: &[Expression],
    ) -> Result<(), CompileError> {
        let is_super = matches!(
            receiver,
            Expression::PseudoVar {
                value: PseudoVar::Super,
                ..
            }
        );
        if is_super {
            self.bytecodes.push(PUSH_SELF);
        } else {
            self.compile_expression(receiver)?;
        }
        for argument in arguments {
            self.compile_expression(argument)?;
        }
        self.emit_selector_send(selector, is_super)
    }

    fn compile_cascade(
        &mut self,
        head: &Expression,
        messages: &[CascadeMessage],
    ) -> Result<(), CompileError> {
        let (receiver, selector, arguments) = match head {
            Expression::Send {
                receiver,
                selector,
                arguments,
                ..
            } => (receiver.as_ref(), selector.as_str(), arguments.as_slice()),
            _ => return Err(CompileError::InvalidCascade),
        };
        if matches!(receiver, Expression::PseudoVar { value: PseudoVar::Super, .. }) {
            return Err(CompileError::Unsupported("cascade on super send"));
        }

        self.compile_expression(receiver)?;
        let total_messages = messages.len() + 1;
        for message_index in 0..total_messages {
            let (selector, arguments) = if message_index == 0 {
                (selector, arguments)
            } else {
                let message = &messages[message_index - 1];
                (message.selector.as_str(), message.arguments.as_slice())
            };
            if message_index + 1 < total_messages {
                self.bytecodes.push(DUP);
            }
            for argument in arguments {
                self.compile_expression(argument)?;
            }
            self.emit_selector_send(selector, false)?;
            if message_index + 1 < total_messages {
                self.bytecodes.push(POP);
            }
        }
        Ok(())
    }

    fn compile_block(&mut self, block: &Block) -> Result<(), CompileError> {
        let captured_names = collect_captured_names(block, &self.locals, &self.captured_names)?;
        for name in &captured_names {
            match self.resolve_name(name) {
                ResolvedName::Local(index) | ResolvedName::Captured(index) => self.emit_push_temp(index)?,
                ResolvedName::InstVar(_) | ResolvedName::GlobalAssociation(_) | ResolvedName::GlobalValue(_) => {
                    return Err(CompileError::Unsupported("only local variable capture is supported"));
                }
            }
        }

        let block_method = UnitCompiler::for_block(self.vm, self.class_index, block, &captured_names).compile()?;
        let literal_index = self.add_literal(block_method);
        self.bytecodes.push(PUSH_CLOSURE);
        self.bytecodes.push(block.args.len() as u8);
        self.bytecodes.push(captured_names.len() as u8);
        self.bytecodes.push(0);
        self.bytecodes.push(0);
        if literal_index >= 256 {
            return Err(CompileError::InvalidClosureLiteralIndex(literal_index));
        }
        self.bytecodes.push(literal_index as u8);
        Ok(())
    }

    fn resolve_name(&mut self, name: &str) -> ResolvedName {
        if let Some(index) = self.locals.get(name).copied() {
            if let Some(captured_index) = self.captured_names.get(name).copied() {
                ResolvedName::Captured(captured_index)
            } else {
                ResolvedName::Local(index)
            }
        } else if let Some(index) = self.vm.class_table.instance_variable_index(self.class_index, name) {
            ResolvedName::InstVar(index)
        } else if let Some(class_oop) = self
            .vm
            .class_table
            .iter()
            .find_map(|(_, info)| (info.name == name).then_some(info.oop))
        {
            ResolvedName::GlobalValue(class_oop)
        } else {
            ResolvedName::GlobalAssociation(self.vm.global_association(name))
        }
    }

    fn emit_literal(&mut self, literal: &Literal) -> Result<(), CompileError> {
        match literal {
            Literal::Integer(-1) => self.emit_small_int_short(-1),
            Literal::Integer(0) => self.emit_small_int_short(0),
            Literal::Integer(1) => self.emit_small_int_short(1),
            Literal::Integer(2) => self.emit_small_int_short(2),
            Literal::Integer(value @ 0..=255) => {
                self.bytecodes.push(PUSH_SMALL_INT_EXT);
                self.bytecodes.push(*value as u8);
                Ok(())
            }
            Literal::Integer(value) => {
                let oop = Oop::from_i64(*value).ok_or(CompileError::Unsupported("large integer literal"))?;
                let index = self.add_literal(oop);
                self.emit_push_literal(index)
            }
            Literal::String(value) => {
                let oop = self
                    .vm
                    .heap
                    .allocate_bytes_in(CLASS_INDEX_STRING, value.as_bytes(), Generation::Old);
                let index = self.add_literal(oop);
                self.emit_push_literal(index)
            }
            Literal::Symbol(value) => {
                let oop = self.vm.intern_symbol(value);
                let index = self.add_literal(oop);
                self.emit_push_literal(index)
            }
            Literal::LiteralArray(values) => {
                let mut oops = Vec::with_capacity(values.len());
                for value in values {
                    oops.push(self.literal_oop(value)?);
                }
                let oop = self.vm.make_array(&oops);
                let index = self.add_literal(oop);
                self.emit_push_literal(index)
            }
            Literal::Nil => {
                self.bytecodes.push(PUSH_NIL);
                Ok(())
            }
            Literal::True => {
                self.bytecodes.push(PUSH_TRUE);
                Ok(())
            }
            Literal::False => {
                self.bytecodes.push(PUSH_FALSE);
                Ok(())
            }
        }
    }

    fn literal_oop(&mut self, literal: &Literal) -> Result<Oop, CompileError> {
        match literal {
            Literal::Integer(value) => Oop::from_i64(*value)
                .ok_or(CompileError::Unsupported("large integer literal")),
            Literal::String(value) => Ok(self
                .vm
                .heap
                .allocate_bytes_in(CLASS_INDEX_STRING, value.as_bytes(), Generation::Old)),
            Literal::Symbol(value) => Ok(self.vm.intern_symbol(value)),
            Literal::LiteralArray(values) => {
                let mut items = Vec::with_capacity(values.len());
                for value in values {
                    items.push(self.literal_oop(value)?);
                }
                Ok(self.vm.make_array(&items))
            }
            Literal::Nil => Ok(Oop::nil()),
            Literal::True => Ok(self.vm.true_oop()),
            Literal::False => Ok(self.vm.false_oop()),
        }
    }

    fn emit_selector_send(&mut self, selector: &str, is_super: bool) -> Result<(), CompileError> {
        if !is_super {
            if let Some(opcode) = special_send_opcode(selector) {
                self.bytecodes.push(opcode);
                return Ok(());
            }
        }
        let selector_oop = self.vm.intern_symbol(selector);
        let selector_literal = self.add_literal(selector_oop);
        self.emit_send(selector_literal, is_super)
    }

    fn emit_small_int_short(&mut self, value: i64) -> Result<(), CompileError> {
        self.bytecodes.push(match value {
            -1 => PUSH_MINUS_ONE,
            0 => PUSH_ZERO,
            1 => PUSH_ONE,
            2 => PUSH_TWO,
            _ => return Err(CompileError::Unsupported("unsupported short integer literal")),
        });
        Ok(())
    }

    fn emit_push_temp(&mut self, index: usize) -> Result<(), CompileError> {
        if index < 16 {
            self.bytecodes.push(PUSH_TEMP_BASE + index as u8);
        } else if index < 256 {
            self.bytecodes.push(PUSH_TEMP_EXT);
            self.bytecodes.push(index as u8);
        } else {
            return Err(CompileError::InvalidTempIndex(index));
        }
        Ok(())
    }

    fn emit_pop_store_temp(&mut self, index: usize) -> Result<(), CompileError> {
        if index < 8 {
            self.bytecodes.push(POP_STORE_TEMP_BASE + index as u8);
        } else if index < 256 {
            self.bytecodes.push(POP_STORE_TEMP_EXT);
            self.bytecodes.push(index as u8);
        } else {
            return Err(CompileError::InvalidTempIndex(index));
        }
        Ok(())
    }

    fn emit_push_inst_var(&mut self, index: usize) -> Result<(), CompileError> {
        if index < 16 {
            self.bytecodes.push(PUSH_INST_VAR_BASE + index as u8);
        } else if index < 256 {
            self.bytecodes.push(PUSH_INST_VAR_EXT);
            self.bytecodes.push(index as u8);
        } else {
            return Err(CompileError::InvalidInstVarIndex(index));
        }
        Ok(())
    }

    fn emit_pop_store_inst_var(&mut self, index: usize) -> Result<(), CompileError> {
        if index < 8 {
            self.bytecodes.push(POP_STORE_INST_VAR_BASE + index as u8);
        } else if index < 256 {
            self.bytecodes.push(POP_STORE_INST_VAR_EXT);
            self.bytecodes.push(index as u8);
        } else {
            return Err(CompileError::InvalidInstVarIndex(index));
        }
        Ok(())
    }

    fn emit_push_literal(&mut self, index: usize) -> Result<(), CompileError> {
        if index < 16 {
            self.bytecodes.push(PUSH_LITERAL_BASE + index as u8);
        } else if index < 256 {
            self.bytecodes.push(PUSH_LITERAL_EXT);
            self.bytecodes.push(index as u8);
        } else {
            return Err(CompileError::InvalidLiteralIndex(index));
        }
        Ok(())
    }

    fn emit_push_lit_var(&mut self, index: usize) -> Result<(), CompileError> {
        if index < 16 {
            self.bytecodes.push(PUSH_LIT_VAR_BASE + index as u8);
        } else if index < 256 {
            self.bytecodes.push(PUSH_LIT_VAR_EXT);
            self.bytecodes.push(index as u8);
        } else {
            return Err(CompileError::InvalidLiteralIndex(index));
        }
        Ok(())
    }

    fn emit_send(&mut self, literal_index: usize, is_super: bool) -> Result<(), CompileError> {
        if literal_index < 16 && !is_super {
            self.bytecodes.push(SEND_SHORT_BASE + literal_index as u8);
        } else if literal_index < 256 {
            self.bytecodes.push(if is_super { SUPER_SEND_EXT } else { SEND_EXT });
            self.bytecodes.push(literal_index as u8);
        } else {
            return Err(CompileError::InvalidSelectorLiteralIndex(literal_index));
        }
        Ok(())
    }

    fn add_literal(&mut self, oop: Oop) -> usize {
        if let Some(index) = self.literals.iter().position(|existing| *existing == oop) {
            index
        } else {
            let index = self.literals.len();
            self.literals.push(oop);
            index
        }
    }
}

fn collect_captured_names(
    block: &Block,
    outer_locals: &HashMap<String, usize>,
    parent_captured: &HashMap<String, usize>,
) -> Result<Vec<String>, CompileError> {
    let mut local_names = HashSet::new();
    for name in &block.args {
        local_names.insert(name.clone());
    }
    for name in &block.temps {
        local_names.insert(name.clone());
    }
    let mut captured = Vec::new();
    collect_captured_from_statements(
        &block.statements,
        outer_locals,
        parent_captured,
        &local_names,
        &mut captured,
    )?;
    Ok(captured)
}

fn collect_captured_from_statements(
    statements: &[Statement],
    outer_locals: &HashMap<String, usize>,
    parent_captured: &HashMap<String, usize>,
    local_names: &HashSet<String>,
    captured: &mut Vec<String>,
) -> Result<(), CompileError> {
    for statement in statements {
        match statement {
            Statement::Expression(expr) => {
                collect_captured_from_expression(expr, outer_locals, parent_captured, local_names, captured)?;
            }
            Statement::Assignment { name, value, .. } => {
                if !local_names.contains(name)
                    && (outer_locals.contains_key(name) || parent_captured.contains_key(name))
                    && !captured.iter().any(|existing| existing == name)
                {
                    captured.push(name.clone());
                }
                collect_captured_from_expression(value, outer_locals, parent_captured, local_names, captured)?;
            }
            Statement::Return { value, .. } => {
                collect_captured_from_expression(value, outer_locals, parent_captured, local_names, captured)?;
            }
        }
    }
    Ok(())
}

fn collect_captured_from_expression(
    expr: &Expression,
    outer_locals: &HashMap<String, usize>,
    parent_captured: &HashMap<String, usize>,
    local_names: &HashSet<String>,
    captured: &mut Vec<String>,
) -> Result<(), CompileError> {
    match expr {
        Expression::Literal { .. } | Expression::PseudoVar { .. } => {}
        Expression::Variable { name, .. } => {
            if !local_names.contains(name)
                && (outer_locals.contains_key(name) || parent_captured.contains_key(name))
                && !captured.iter().any(|existing| existing == name)
            {
                captured.push(name.clone());
            }
        }
        Expression::Send { receiver, arguments, .. } => {
            collect_captured_from_expression(receiver, outer_locals, parent_captured, local_names, captured)?;
            for argument in arguments {
                collect_captured_from_expression(argument, outer_locals, parent_captured, local_names, captured)?;
            }
        }
        Expression::Cascade { head, messages, .. } => {
            collect_captured_from_expression(head, outer_locals, parent_captured, local_names, captured)?;
            for message in messages {
                for argument in &message.arguments {
                    collect_captured_from_expression(argument, outer_locals, parent_captured, local_names, captured)?;
                }
            }
        }
        Expression::Block(block) => {
            let mut nested_local_names = local_names.clone();
            for name in &block.args {
                nested_local_names.insert(name.clone());
            }
            for name in &block.temps {
                nested_local_names.insert(name.clone());
            }
            collect_captured_from_statements(
                &block.statements,
                outer_locals,
                parent_captured,
                &nested_local_names,
                captured,
            )?;
        }
    }
    Ok(())
}

fn collect_boxed_local_names(
    statements: &[Statement],
    local_names: &[String],
    initially_shadowed: &HashSet<String>,
) -> HashSet<String> {
    let current_locals = local_names.iter().cloned().collect::<HashSet<_>>();
    let mut boxed = HashSet::new();
    for statement in statements {
        collect_boxed_from_statement(statement, &current_locals, initially_shadowed, &mut boxed);
    }
    boxed
}

fn collect_boxed_from_statement(
    statement: &Statement,
    current_locals: &HashSet<String>,
    shadowed: &HashSet<String>,
    boxed: &mut HashSet<String>,
) {
    match statement {
        Statement::Expression(expr) => {
            collect_boxed_from_expression(expr, current_locals, shadowed, boxed)
        }
        Statement::Assignment { name, value, .. } => {
            if current_locals.contains(name) && !shadowed.contains(name) {
                boxed.insert(name.clone());
            }
            collect_boxed_from_expression(value, current_locals, shadowed, boxed);
        }
        Statement::Return { value, .. } => {
            collect_boxed_from_expression(value, current_locals, shadowed, boxed)
        }
    }
}

fn collect_boxed_from_expression(
    expr: &Expression,
    current_locals: &HashSet<String>,
    shadowed: &HashSet<String>,
    boxed: &mut HashSet<String>,
) {
    match expr {
        Expression::Literal { .. } | Expression::PseudoVar { .. } => {}
        Expression::Variable { .. } => {}
        Expression::Send { receiver, arguments, .. } => {
            collect_boxed_from_expression(receiver, current_locals, shadowed, boxed);
            for argument in arguments {
                collect_boxed_from_expression(argument, current_locals, shadowed, boxed);
            }
        }
        Expression::Cascade { head, messages, .. } => {
            collect_boxed_from_expression(head, current_locals, shadowed, boxed);
            for message in messages {
                for argument in &message.arguments {
                    collect_boxed_from_expression(argument, current_locals, shadowed, boxed);
                }
            }
        }
        Expression::Block(block) => {
            let mut nested_shadowed = shadowed.clone();
            for name in &block.args {
                nested_shadowed.insert(name.clone());
            }
            for name in &block.temps {
                nested_shadowed.insert(name.clone());
            }
            for statement in &block.statements {
                collect_boxed_capture_uses(statement, current_locals, &nested_shadowed, boxed);
            }
        }
    }
}

fn collect_boxed_capture_uses(
    statement: &Statement,
    current_locals: &HashSet<String>,
    shadowed: &HashSet<String>,
    boxed: &mut HashSet<String>,
) {
    match statement {
        Statement::Expression(expr) => collect_boxed_capture_expr(expr, current_locals, shadowed, boxed),
        Statement::Assignment { name, value, .. } => {
            if current_locals.contains(name) && !shadowed.contains(name) {
                boxed.insert(name.clone());
            }
            collect_boxed_capture_expr(value, current_locals, shadowed, boxed);
        }
        Statement::Return { value, .. } => collect_boxed_capture_expr(value, current_locals, shadowed, boxed),
    }
}

fn collect_boxed_capture_expr(
    expr: &Expression,
    current_locals: &HashSet<String>,
    shadowed: &HashSet<String>,
    boxed: &mut HashSet<String>,
) {
    match expr {
        Expression::Literal { .. } | Expression::PseudoVar { .. } => {}
        Expression::Variable { name, .. } => {
            if current_locals.contains(name) && !shadowed.contains(name) {
                boxed.insert(name.clone());
            }
        }
        Expression::Send { receiver, arguments, .. } => {
            collect_boxed_capture_expr(receiver, current_locals, shadowed, boxed);
            for argument in arguments {
                collect_boxed_capture_expr(argument, current_locals, shadowed, boxed);
            }
        }
        Expression::Cascade { head, messages, .. } => {
            collect_boxed_capture_expr(head, current_locals, shadowed, boxed);
            for message in messages {
                for argument in &message.arguments {
                    collect_boxed_capture_expr(argument, current_locals, shadowed, boxed);
                }
            }
        }
        Expression::Block(block) => {
            let mut deeper_shadowed = shadowed.clone();
            for name in &block.args {
                deeper_shadowed.insert(name.clone());
            }
            for name in &block.temps {
                deeper_shadowed.insert(name.clone());
            }
            for statement in &block.statements {
                collect_boxed_capture_uses(statement, current_locals, &deeper_shadowed, boxed);
            }
        }
    }
}

fn special_send_opcode(selector: &str) -> Option<u8> {
    let index = match selector {
        "+" => 0,
        "-" => 1,
        "*" => 2,
        "/" => 3,
        "<" => 4,
        ">" => 5,
        "<=" => 6,
        ">=" => 7,
        "=" => 8,
        "~=" => 9,
        "bitAnd:" => 10,
        "bitOr:" => 11,
        "bitShift:" => 12,
        "@" => 13,
        "at:" => 14,
        "at:put:" => 15,
        _ => return None,
    };
    Some(SEND_SPECIAL_BASE + index)
}

#[cfg(test)]
mod tests {
    use crate::{
        class_table::{CLASS_INDEX_TRUE, CLASS_INDEX_UNDEFINED_OBJECT},
        compiler::{compile_doit, compile_method_source},
        value::Oop,
        Vm,
    };

    #[test]
    fn compiles_and_installs_simple_method() {
        let mut vm = Vm::new();
        compile_method_source(&mut vm, CLASS_INDEX_TRUE, "answer ^ 1 + 2").unwrap();
        let selector = vm.intern_symbol("answer");
        let result = vm.send(vm.true_oop(), selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
    }

    #[test]
    fn compiles_args_and_temp_assignment() {
        let mut vm = Vm::new();
        compile_method_source(
            &mut vm,
            CLASS_INDEX_TRUE,
            "increment: x | y | y := x + 1. ^ y",
        )
        .unwrap();
        let selector = vm.intern_symbol("increment:");
        let result = vm.send(vm.true_oop(), selector, &[Oop::from_i64(41).unwrap()]).unwrap();
        assert_eq!(result.as_i64(), Some(42));
    }

    #[test]
    fn compiles_instance_variable_access() {
        let mut vm = Vm::new();
        let point = vm
            .new_class("Point", Some(crate::class_table::CLASS_INDEX_BEHAVIOR), crate::Format::FixedPointers, 2)
            .unwrap();
        vm.set_instance_variables(point, vec!["x".to_string(), "y".to_string()])
            .unwrap();
        compile_method_source(&mut vm, point, "x ^ x").unwrap();
        compile_method_source(&mut vm, point, "x: value x := value").unwrap();
        let obj = vm.new_instance(point, 0).unwrap();
        let set_selector = vm.intern_symbol("x:");
        let get_selector = vm.intern_symbol("x");
        let _ = vm.send(obj, set_selector, &[Oop::from_i64(7).unwrap()]).unwrap();
        let result = vm.send(obj, get_selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(7));
    }

    #[test]
    fn compiles_global_reads() {
        let mut vm = Vm::new();
        vm.set_global("Answer", Oop::from_i64(42).unwrap());
        let method = compile_doit(&mut vm, "Answer").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(42));
    }

    #[test]
    fn compiles_blocks_with_copied_values() {
        let mut vm = Vm::new();
        compile_method_source(
            &mut vm,
            CLASS_INDEX_TRUE,
            "makeAdder: x ^ [:y | x + y]",
        )
        .unwrap();
        let selector = vm.intern_symbol("makeAdder:");
        let closure = vm.send(vm.true_oop(), selector, &[Oop::from_i64(5).unwrap()]).unwrap();
        let value_selector = vm.intern_symbol("value:");
        let result = vm.send(closure, value_selector, &[Oop::from_i64(3).unwrap()]).unwrap();
        assert_eq!(result.as_i64(), Some(8));
    }

    #[test]
    fn compiles_non_local_returns_from_blocks() {
        let mut vm = Vm::new();
        compile_method_source(&mut vm, CLASS_INDEX_TRUE, "escape ^ [^ 7] value. 9").unwrap();
        let selector = vm.intern_symbol("escape");
        let result = vm.send(vm.true_oop(), selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(7));
    }

    #[test]
    fn compiles_cascades() {
        let mut vm = Vm::new();
        let point = vm
            .new_class(
                "CascadePoint",
                Some(crate::class_table::CLASS_INDEX_BEHAVIOR),
                crate::Format::FixedPointers,
                1,
            )
            .unwrap();
        vm.set_instance_variables(point, vec!["x".to_string()]).unwrap();
        compile_method_source(&mut vm, point, "x ^ x").unwrap();
        compile_method_source(&mut vm, point, "x: value x := value").unwrap();
        vm.set_global("CascadePoint", vm.class_table.class_oop(point).unwrap());
        let method = compile_doit(&mut vm, "CascadePoint new x: 41; x").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(41));
    }

    #[test]
    fn compiles_doits() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "1 + 2").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
        let _ = CLASS_INDEX_UNDEFINED_OBJECT;
    }

    #[test]
    fn compiles_doits_with_temps() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "| x | x := 1. x + 2").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
    }

    #[test]
    fn compiles_global_assignments_in_doits() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "Answer := 42. Answer").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(42));
        assert_eq!(vm.global_value("Answer").and_then(Oop::as_i64), Some(42));
    }

    #[test]
    fn compiles_this_context() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "thisContext").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(vm.class_of(result).unwrap(), crate::class_table::CLASS_INDEX_METHOD_CONTEXT);
    }

    #[test]
    fn compiles_super_sends() {
        let mut vm = Vm::new();
        let base = vm
            .new_class(
                "BaseNumber",
                Some(crate::class_table::CLASS_INDEX_BEHAVIOR),
                crate::Format::FixedPointers,
                0,
            )
            .unwrap();
        let derived = vm
            .new_class("DerivedNumber", Some(base), crate::Format::FixedPointers, 0)
            .unwrap();
        compile_method_source(&mut vm, base, "value ^ 1").unwrap();
        compile_method_source(&mut vm, derived, "value ^ super value + 1").unwrap();
        let receiver = vm.new_instance(derived, 0).unwrap();
        let selector = vm.intern_symbol("value");
        let result = vm.send(receiver, selector, &[]).unwrap();
        assert_eq!(result.as_i64(), Some(2));
    }

    #[test]
    fn compiles_captured_temp_assignment() {
        let mut vm = Vm::new();
        let method = compile_doit(&mut vm, "| x | x := 1. [ x := x + 2 ] value. x").unwrap();
        let result = vm.run_method(method, Oop::nil(), &[]).unwrap();
        assert_eq!(result.as_i64(), Some(3));
    }
}
