// импорты
use std::collections::VecDeque;
use scopeguard::defer;
use crate::error;
use crate::errors::errors::{Error};
use crate::lexer::address::Address;
use crate::vm::bytecode::{Chunk, Opcode, OpcodeValue};
use crate::vm::flow::ControlFlow;
use crate::vm::natives::natives;
use crate::vm::values::{FnOwner, Function, Instance, Symbol, Trait, TraitFn, Type, Unit, Value};
use crate::vm::memory::gc::GC;
use crate::vm::memory::memory;
use crate::vm::stack::table::Table;
use crate::vm::stack::table_stack::{StackFrame, TableStack};

// настройки
#[derive(Debug)]
pub struct VmSettings {
    gc_threshold: usize,
    gc_debug: bool,
}
// имплементация
impl VmSettings {
    pub fn new(gc_threshold: usize, gc_debug: bool) -> Self {
        Self { gc_threshold, gc_debug }
    }
}

// вм
#[derive(Debug)]
pub struct VM {
    types: *mut Table,
    pub units: *mut Table,
    traits: *mut Table,
    pub natives: *mut Table,
    pub gc: *mut GC,
    settings: VmSettings,
    pub evaluation_stack: VecDeque<Value>,
    pub table_stack: *mut TableStack
}
// имплементация вм
#[allow(non_upper_case_globals)]
#[allow(unused_qualifications)]
impl VM {
    // новая вм
    pub unsafe fn new(settings: VmSettings) -> VM {
        // вм
        let mut vm = VM {
            types: memory::alloc_value(Table::new()),
            units: memory::alloc_value(Table::new()),
            traits: memory::alloc_value(Table::new()),
            natives: memory::alloc_value(Table::new()),
            gc: memory::alloc_value(GC::new(settings.gc_debug)),
            evaluation_stack: VecDeque::new(),
            table_stack: memory::alloc_value(TableStack::new()),
            settings
        };
        // нативы
        if let Err(e) = natives::provide_builtins(&mut vm) {
            error!(e)
        }
        // возвращаем
        vm
    }

    // пуш
    pub unsafe fn push(&mut self, value: Value) {
        self.evaluation_stack.push_back(value);
    }

    // поп
    pub fn pop(&mut self, address: &Address) -> Result<Value, ControlFlow> {
        if self.evaluation_stack.is_empty() {
            error!(Error::new(
                address.clone(),
                "stack underflow.".to_string(),
                "check your code.".to_string()
            ));
        }
        Ok(self.evaluation_stack.pop_back().unwrap())
    }

    // shallow очистка
    pub unsafe fn cleanup(&mut self) {
        // todo: add vm debug option
        // высвобождаем типы
        (*self.types).free();
        memory::free_value(self.types);
        // высвобождаем трэйты
        (*self.traits).free();
        memory::free_value(self.traits);
        // высвобождаем нативные функции
        (*self.natives).free();
        memory::free_value(self.natives);
        // высвобождаем таблицу юнитов
        memory::free_value(self.units);
        // высвобождаем gc
        memory::free_value(self.gc);
    }

    // очистка мусора
    pub unsafe fn gc_invoke(&mut self) {
        (*self.gc).collect_garbage(self);
    }

    // добавление в учет сборщика мусора
    pub unsafe fn gc_register(&mut self, value: Value) {
        // gil
        // добавляем объект
        (*self.gc).add_object(value);
        // проверяем порог gc
        if (*self.gc).objects_amount() > self.settings.gc_threshold {
            // вызываем gc
            self.gc_invoke();
            // увеличиваем порог
            self.settings.gc_threshold *= 2;
        }
    }

    // пуш в стек
    pub unsafe fn op_push(&mut self, value: OpcodeValue) -> Result<(), ControlFlow> {
        // проверяем значение
        match value {
            OpcodeValue::Int(int) => { self.push(Value::Int(int)); }
            OpcodeValue::Float(float) => { self.push(Value::Float(float)); }
            OpcodeValue::Bool(bool) => { self.push(Value::Bool(bool)); }
            OpcodeValue::String(string) => {
                let new_string = Value::String(
                    memory::alloc_value(
                        string
                    )
                );
                self.gc_register(new_string);
                self.push(new_string);
            }
            OpcodeValue::Raw(raw) => {
                match raw {
                    Value::Instance(_) | Value::Fn(_) |
                    Value::Native(_) | Value::String(_) |
                    Value::Unit(_) | Value::List(_) => {
                        // добавляем в gc
                        self.gc_register(raw);
                        // пушим
                        self.push(raw);
                    }
                    _ => {
                        // пушим
                        self.push(raw);
                    }
                }
            }
        }
        // успех
        Ok(())
    }
    
    // бинарная операция
    unsafe fn op_binary(&mut self, address: &Address, op: &str) -> Result<(), ControlFlow> {
        // два операнда
        let operand_a = self.pop(&address)?;
        let operand_b = self.pop(&address)?;
        // ошибка
        let error = Error::new(
            address.clone(),
            format!("could not use '{}' with {:?} and {:?}", op, operand_a, operand_b),
            "check your code.".to_string()
        );
        // бинарная операция
        match op {
            "+" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float(a + b)); }
                        Value::Int(b) => { self.push(Value::Float(a + (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float((a as f64) + b)); }
                        Value::Int(b) => { self.push(Value::Int(a + b)); }
                        _ => { error!(error); }
                    }}
                    Value::String(a) => {
                        let string = Value::String(
                            memory::alloc_value(format!("{}{:?}", *a, operand_b))
                        );
                        self.push(string);
                        self.gc_register(string);
                    }
                    _ => { error!(error); }
                }
            }
            "-" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float(a - b)); }
                        Value::Int(b) => { self.push(Value::Float(a - (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float((a as f64) - b)); }
                        Value::Int(b) => { self.push(Value::Int(a - b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            }
            "*" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float(a * b)); }
                        Value::Int(b) => { self.push(Value::Float(a * (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float((a as f64) * b)); }
                        Value::Int(b) => { self.push(Value::Int(a * b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            }
            "/" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float(a / b)); }
                        Value::Int(b) => { self.push(Value::Float(a / (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float((a as f64) / b)); }
                        Value::Int(b) => { self.push(Value::Int(a / b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            }
            "%" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float(a % b)); }
                        Value::Int(b) => { self.push(Value::Float(a % (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Float((a as f64) % b)); }
                        Value::Int(b) => { self.push(Value::Int(a % b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            }
            _ => { panic!("operator = {} is not found.", op)}
        }
        Ok(())
    }

    // негэйт
    unsafe fn op_negate(&mut self, address: &Address) -> Result<(), ControlFlow> {
        // операнд
        let operand = self.pop(&address)?;
        // негэйт
        match operand {
            Value::Float(a) => {
                self.push(Value::Float(-a));
            }
            Value::Int(a) => {
                self.push(Value::Int(-a));
            }
            _ => {
                // ошибка
                error!(Error::new(
                    address.clone(),
                    format!("could not use 'negate' for {:?}", operand),
                    "check your code.".to_string()
                ));
            }
        }
        // успех
        Ok(())
    }

    // бэнг
    unsafe fn op_bang(&mut self, address: &Address) -> Result<(), ControlFlow> {
        // операнд
        let operand = self.pop(address)?;
        // бэнг
        match operand {
            Value::Bool(b) => {
                self.push(Value::Bool(!b));
            }
            _ => { 
                error!(Error::new(
                    address.clone(),
                    format!("could not use 'bang' for {:?}", operand),
                    "check your code.".to_string()
                ));
            }
        }
        // успех
        Ok(())
    }

    // условие
    unsafe fn op_conditional(&mut self, address: &Address, op: &str) -> Result<(), ControlFlow> {
        // операнды
        let operand_a = self.pop(&address)?;
        let operand_b = self.pop(&address)?;
        let error = Error::new(
            address.clone(),
            format!("could not use '{}' for {:?} and {:?}", op, operand_a, operand_b),
            "check your code.".to_string()
        );
        // условие
        match op {
            ">" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool(a > b)); }
                        Value::Int(b) => { self.push(Value::Bool(a > (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool((a as f64) > b)); }
                        Value::Int(b) => { self.push(Value::Bool(a > b)); }
                        _ => { error!(error); }
                    }}
                    Value::String(a) => { match operand_b {
                        Value::String(b) => { self.push(Value::Bool(*a > *b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            },
            "<" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool(a < b)); }
                        Value::Int(b) => { self.push(Value::Bool(a < (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool((a as f64) < b)); }
                        Value::Int(b) => { self.push(Value::Bool(a < b)); }
                        _ => { error!(error); }
                    }}
                    Value::String(a) => { match operand_b {
                        Value::String(b) => { self.push(Value::Bool(*a < *b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            },
            ">=" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool(a >= b)); }
                        Value::Int(b) => { self.push(Value::Bool(a >= (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool((a as f64) >= b)); }
                        Value::Int(b) => { self.push(Value::Bool(a >= b)); }
                        _ => { error!(error); }
                    }}
                    Value::String(a) => { match operand_b {
                        Value::String(b) => { self.push(Value::Bool(*a >= *b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            }
            "<=" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool(a <= b)); }
                        Value::Int(b) => { self.push(Value::Bool(a <= (b as f64))); }
                        _ => { error!(error); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool((a as f64) <= b)); }
                        Value::Int(b) => { self.push(Value::Bool(a <= b)); }
                        _ => { error!(error); }
                    }}
                    Value::String(a) => { match operand_b {
                        Value::String(b) => { self.push(Value::Bool(*a <= *b)); }
                        _ => { error!(error); }
                    }}
                    _ => { error!(error); }
                }
            }
            "==" => {
                match operand_a {
                    Value::Float(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool(a == b)); }
                        Value::Int(b) => { self.push(Value::Bool(a == (b as f64))); }
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::Int(a) => { match operand_b {
                        Value::Float(b) => { self.push(Value::Bool((a as f64) == b)); }
                        Value::Int(b) => { self.push(Value::Bool(a == b)); }
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::Null => { match operand_b {
                        Value::Null => { self.push(Value::Bool(true)); }
                        _ => { self.push(Value::Bool(false));  }
                    }}
                    Value::Fn(f1) => { match operand_b {
                        Value::Fn(f2) => { self.push(Value::Bool(f1 == f2)); }
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::Bool(a) => { match operand_b {
                        Value::Bool(b) => { self.push(Value::Bool(a == b)); }
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::Instance(a) => { match operand_b {
                        Value::Instance(b) => { self.push(Value::Bool(a == b)); }
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::Type(a) => { match operand_b {
                        Value::Type(b) => { self.push(Value::Bool(a == b))}
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::String(a) => { match operand_b {
                        Value::String(b) => { self.push(Value::Bool(*a == *b)) }
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::Native(a) => { match operand_b {
                        Value::Native(b) => { self.push(Value::Bool(a == b))}
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    Value::Trait(a) => { match operand_b {
                        Value::Trait(b) => { self.push(Value::Bool(a == b))}
                        _ => { self.push(Value::Bool(false)); }
                    }}
                    _ => {
                        self.push(Value::Bool(false));
                    }
                }
            }
            "!=" => {
                // операнды
                self.push(operand_b);
                self.push(operand_a);
                // выполняем ==
                self.op_conditional(&address, "==")?;
                // инверсируем
                self.op_bang(&address)?;
            }
            _ => { panic!("operator {} is not found.", op)}
        }
        // успех
        Ok(())
    }

    // логика
    unsafe fn op_logical(&mut self, address: &Address, op: &str) -> Result<(), ControlFlow> {
        // операнды
        let operand_a = self.pop(&address)?;
        let operand_b = self.pop(&address)?;
        let error = Error::new(
            address.clone(),
            format!("could not use '{}' for {:?} and {:?}", op, operand_a, operand_b),
            "check your code.".to_string()
        );
        // логика
        match op {
            "and" => {
                match operand_a {
                    Value::Bool(a) => {
                        match operand_b {
                            Value::Bool(b) => { self.push(Value::Bool(a && b)); }
                            _ => { error!(error); }
                        }
                    }
                    _ => { error!(error); }
                }
            }
            "or" => {
                match operand_a {
                    Value::Bool(a) => {
                        match operand_b {
                            Value::Bool(b) => { self.push(Value::Bool(a || b)); }
                            _ => { error!(error); }
                        }
                    }
                    _ => { error!(error); }
                }
            }
            _ => { panic!("operator = {} is not found.", op)}
        }
        // успех
        Ok(())
    }

    // иф
    unsafe fn op_if(&mut self, addr: &Address, cond: &Chunk,
                    body: &Chunk, elif: &Option<Box<Opcode>>) -> Result<(), ControlFlow> {
        // стэк таблиц
        let table_stack = self.table_stack;
        // таблица
        (*table_stack).push_frame(None);
        // высвобождение
        defer! {
            // удаляем таблицу
            (*table_stack).pop_frame(addr);
        }
        // условие
        self.run(cond)?;
        let bool = self.pop(&addr)?;
        // проверка
        if let Value::Bool(b) = bool {
            if b {
                self.run(body)?
            } else {
                if let Option::Some(else_if) = elif {
                    self.run(&Chunk::of(*else_if.clone()))? // todo: chunk::of has high runtime cost!
                }
            }
        } else {
            error!(Error::new(
                addr.clone(),
                format!("condition provided not a bool: {:?}", bool),
                "condition should provide a bool.".to_string()
            ))
        }
        // успех
        Ok(())
    }

    // луп
    #[allow(unused_variables)]
    unsafe fn op_loop(&mut self, addr: &Address, body: &Chunk) -> Result<(), ControlFlow> {
        // стэк таблиц
        let table_stack = self.table_stack;
        // таблица
        (*table_stack).push_frame(None);
        // высвобождение
        defer! {
            // удаляем таблицу
            (*table_stack).pop_frame(addr);
        }
        // проверка
        loop {
            if let Err(e) = self.run(&body) {
                match e {
                    ControlFlow::Continue => {
                        continue;
                    },
                    ControlFlow::Break => {
                        break;
                    }
                    _ => {
                        return Err(e);
                    }
                }
            }
        }
        // успех
        Ok(())
    }

    // дефайн функции
    unsafe fn op_define_fn(&mut self, addr: &Address, symbol: &Symbol,
                           body: &Chunk, params: &Vec<String>) -> Result<(), ControlFlow> {
        // создаём функцию
        let function = memory::alloc_value(
            Function::new(
                symbol.clone(),
                memory::alloc_value(body.clone()),
                params.clone()
            )
        );
        // создаём значение функции и добавляем в gc
        let function_value = Value::Fn(function);
        self.gc_register(function_value,);
        // дефайн функции
        (*self.table_stack).define(&addr, &symbol.name, function_value);
        // дефайн функции по full-name
        if symbol.full_name.is_some() {
            (*self.table_stack).define(&addr, symbol.full_name.as_ref().unwrap(), function_value);
        }
        // успех
        Ok(())
    }

    // бинды функций
    unsafe fn bind_functions(&mut self, table: *mut Table, owner: FnOwner) {
        // биндим
        for val in (*table).fields.values() {
            if let Value::Fn(function) = *val {
                (*function).owner = Some(owner.clone());
            }
            else if let Value::Native(function) = *val {
                (*function).owner = Some(owner.clone());
            }
        }
    }

    // дефайн типа
    unsafe fn op_define_type(&mut self, addr: &Address, symbol: &Symbol, body: &Chunk,
                             constructor: &Vec<String>, impls: &Vec<String>) -> Result<(), ControlFlow> {
        // создаём тип
        let t = memory::alloc_value(
            Type::new(
                symbol.clone(),
                constructor.clone(),
                memory::alloc_value(body.clone()),
                impls.clone()
            )
        );
        // дефайн типа
        if let Err(e) = (*self.types).define(&addr, &symbol.name, Value::Type(t)) {
            error!(e);
        }
        // дефайн по full-name
        if symbol.full_name.is_some() {
            if let Err(e) = (*self.types).define(&addr, symbol.full_name.as_ref().unwrap(), Value::Type(t)){
                error!(e);
            }
        }
        // успех
        Ok(())
    }

    // дефайн юнита
    unsafe fn op_define_unit(&mut self, addr: &Address,
                             symbol: &Symbol, body: &Chunk) -> Result<(), ControlFlow> {
        // создаём юнит
        let unit = memory::alloc_value(
            Unit::new(
                symbol.clone(),
                memory::alloc_value(Table::new())
            )
        );
        // добавляем в учет gc
        self.gc_register(Value::Unit(unit));
        // временно пушим таблицу
        let mut unit_frame = StackFrame::new(None);
        unit_frame.table = (*unit).fields;
        (*self.table_stack).append_frame(unit_frame);
        // временный self
        (*(*unit).fields).fields.insert("self".to_string(), Value::Unit(unit));
        // исполняем тело
        self.run(body)?;
        // удаляем временный self
        (*(*unit).fields).fields.remove(&"self".to_string());
        // бинды
        self.bind_functions((*unit).fields, FnOwner::Unit(unit));
        // удаляем таблицу из стека
        (*self.table_stack).remove_frame();
        // дефайн юнита
        if let Err(e) = (*self.units).define(&addr, &symbol.name, Value::Unit(unit)) {
            error!(e);
        }
        // дефайн по full-name
        if symbol.full_name.is_some() {
            if let Err(e) = (*self.units).define(&addr, symbol.full_name.as_ref().unwrap(), Value::Unit(unit)) {
                error!(e);
            }
        }
        // успех
        Ok(())
    }

    // дефайн тейта
    unsafe fn op_define_trait(&mut self, addr: &Address,
                              symbol: &Symbol, functions: &Vec<TraitFn>) -> Result<(), ControlFlow> {
        // создаём трейт
        let _trait = memory::alloc_value(
            Trait::new(
                symbol.clone(),
                functions.clone()
            )
        );
        // дефайн трейта
        if let Err(e) = (*self.traits).define(&addr, &symbol.name, Value::Trait(_trait)) {
            error!(e);
        }
        // дефайн по full-name
        if symbol.full_name.is_some() {
            if let Err(e) = (*self.traits).define(&addr, symbol.full_name.as_ref().unwrap(), Value::Trait(_trait)) {
                error!(e);
            }
        }
        // успех
        Ok(())
    }

    // дефайн
    unsafe fn op_define(&mut self, addr: &Address, name: &str,
                        has_previous: bool, value: &Chunk) -> Result<(), ControlFlow> {
        // если нет предыдущего
        if !has_previous {
            // исполняем значение
            self.run(value)?;
            // получаем значение
            let operand = self.pop(&addr)?;
            // дефайним
            (*self.table_stack).define(&addr, &name, operand);
        }
        // если есть
        else {
            // получаем значение
            let previous = self.pop(&addr)?;
            // првоеряем
            match previous {
                Value::Instance(instance) => {
                    // исполняем значение
                    self.run(value)?;
                    // получаем значение
                    let operand = self.pop(&addr)?;
                    // дефайним
                    if let Err(e) = (*(*instance).fields).define(&addr, &name, operand) {
                        error!(e);
                    }
                }
                Value::Unit(unit) => {
                    // исполняем значение
                    self.run(value)?;
                    // получаем значение
                    let operand = self.pop(&addr)?;
                    // дефайним
                    if let Err(e) = (*(*unit).fields).define(&addr, &name, operand) {
                        error!(e);
                    }
                }
                _ => {
                    error!(Error::new(
                        addr.clone(),
                        format!("{:?} is not a container.", previous),
                        "you can define variable for unit or instance.".to_string()
                    ))
                }
            }
        }
        // успех
        Ok(())
    }

    // установка значения переменной
    unsafe fn op_set(&mut self, addr: &Address, name: &str, has_previous: bool,
                        value: &Chunk) -> Result<(), ControlFlow> {
        // если нет предыдущего
        if !has_previous {
            // исполняем значение
            self.run(value)?;
            // получаем значение
            let operand = self.pop(addr)?;
            // дефайним
            (*self.table_stack).set(addr, name, operand);
        }
        // если есть
        else {
            // получаем значение
            let previous = self.pop(addr)?;
            // проверяем
            match previous {
                Value::Instance(instance) => {
                    // исполняем значение
                    self.run(value)?;
                    // получаем значение
                    let operand = self.pop(&addr)?;
                    // устанавливаем значение
                    if let Err(e) = (*(*instance).fields).set(&addr, name, operand) {
                        error!(e);
                    }
                }
                Value::Unit(unit) => {
                    // исполняем значение
                    self.run(value)?;
                    // получаем значение
                    let operand = self.pop(&addr)?;
                    // устанавливаем значение
                    if let Err(e) = (*(*unit).fields).set(&addr, name, operand) {
                        error!(e);
                    }
                }
                _ => {
                    error!(Error::new(
                        addr.clone(),
                        format!("{:?} is not a container.", previous),
                        "you can define variable for unit or instance.".to_string()
                    ))
                }
            }
        }
        // успех
        Ok(())
    }

    // загрузка значения переменной
    unsafe fn op_load(&mut self, addr: &Address, name: &str, has_previous: bool, should_push: bool) -> Result<(), ControlFlow> {
        // если нет предыдущего
        if !has_previous {
            // получаем значение
            let lookup_result;
            if (*self.table_stack).exists(&name) {
                lookup_result = (*self.table_stack).find(&addr, &name);
            } else if (*self.types).exists(&name) {
                lookup_result = (*self.types).find(&addr, &name);
            } else {
                lookup_result = (*self.units).find(&addr, &name);
            }
            // проверяем на ошибку
            if let Err(e) = lookup_result {
                // ошибка
                error!(e)
            }
            else if let Ok(value) = lookup_result {
                // пушим в стек
                if !should_push { return Ok(()) }
                self.push(value);
            }
        }
        // если есть
        else {
            // получаем значение
            let previous = self.pop(&addr)?;
            // проверяем
            match previous {
                Value::Instance(instance) => {
                    // получаем значение
                    let lookup_result = (*(*instance).fields).find(&addr, &name);
                    // проверяем на ошибку
                    if let Err(e) = lookup_result {
                        // ошибка
                        error!(e)
                    }
                    else if let Ok(value) = lookup_result {
                        // пушим в стек
                        if !should_push { return Ok(()) }
                        self.push(value);
                    }
                }
                Value::Unit(unit) => {
                    // получаем значение
                    let lookup_result = (*(*unit).fields).find(&addr, &name);
                    // проверяем на ошибку
                    if let Err(e) = lookup_result {
                        // ошибка
                        error!(e)
                    }
                    else if let Ok(value) = lookup_result {
                        // пушим в стек
                        if !should_push { return Ok(()) }
                        self.push(value);
                    }
                }
                _ => {
                    error!(Error::new(
                        addr.clone(),
                        format!("{:?} is not a container.", previous),
                        "you can load variable from unit or instance.".to_string()
                    ))
                }
            }
        }
        // успех
        Ok(())
    }

    // вызов функции
    #[allow(unused_parens)]
    pub unsafe fn call(&mut self, addr: &Address, name: &str,
                       callable: Value, args: &Chunk,
                       should_push: bool) -> Result<(), ControlFlow> {

        // подгрузка аргументов
        unsafe fn pass_arguments(vm: &mut VM, addr: &Address, params: Vec<String>) -> Result<(), ControlFlow> {
            // проходимся по реверсированным параметрам
            for param in params.iter().rev() {
                // получаем аргумент из стека
                let operand = vm.pop(&addr)?;
                // устанавливаем в таблице
                (*vm.table_stack).define(&addr, &param, operand);
            }
            Ok(())
        }

        // только загрузка аргументов
        unsafe fn load_arguments(vm: &mut VM, addr: &Address, name: &str,
                                 params_amount: usize, args: &Chunk) -> Result<(), ControlFlow> {
            // фиксируем размер стека
            let prev_size = vm.evaluation_stack.len();
            // загрузка аргументов
            vm.run(args)?;
            // фиксируем новый размер стека
            let new_size = vm.evaluation_stack.len();
            // количество переданных аргументов
            let passed_amount = new_size-prev_size;
            // проверяем
            if passed_amount == params_amount {
                Ok(())
            } else {
                error!(Error::new(
                    addr.clone(),
                    format!(
                        "invalid args amount: {} to call: {}. stack: {:?}",
                        passed_amount, name, vm.evaluation_stack
                    ),
                    format!("expected {} arguments.", params_amount)
                ));
                Ok(())
            }
        }

        // проверка на функцию
        if let Value::Fn(function) = callable {
            // загрузка аргументов
            load_arguments(
                self,
                addr,
                name,
                (*function).params.len(),
                args
            )?;
            // стэк таблиц
            let table_stack = self.table_stack;
            // создаём таблицу под вызов.
            (*table_stack).push_frame(Some(
                (*function).closure
            ));
            // удаление таблицы
            defer! {
                // удаление таблицы
                (*table_stack).pop_frame(addr)
            }
            // сопоставляем аргументы с параметрами
            pass_arguments(
                self,
                addr,
                (*function).params.clone()
            )?;
            // установка self
            if (*function).owner.is_some() {
                match (*function).owner.clone().unwrap() {
                    FnOwner::Unit(unit) => {
                        (*table_stack).define(&addr, "self", Value::Unit(unit));
                    },
                    FnOwner::Instance(instance) => {
                        (*table_stack).define(&addr, "self", Value::Instance(instance));
                    }
                }
            }
            // вызов
            match self.run(&*(*function).body) {
                // если поймали control flow
                Err(e) => {
                    return match e {
                        // если поймали return
                        ControlFlow::Return(val) => {
                            // пушим
                            if should_push {
                                self.push(val);
                            }
                            // успех
                            Ok(())
                        },
                        // если другая ошибка
                        _ => {
                            // пробрасываем
                            Err(e)
                        }
                    }
                }
                _ => {}
            }
            // успех
            Ok(())
        }
        // проверка на нативную функцию
        else if let Value::Native(function) = callable {
            // загрузка аргументов
            load_arguments(self, &addr, &name, (*function).params_amount, args)?;
            // стэк таблиц
            let table_stack = self.table_stack;
            // создаём таблицу под вызов.
            (*table_stack).push_frame(None);
            // удаление таблицы
            defer! {
                // удаление таблицы
                (*table_stack).pop_frame(addr)
            }
            // self
            if (*function).owner.is_some() {
                match (*function).owner.clone().unwrap() {
                    FnOwner::Unit(unit) => {
                        (*table_stack).define(&addr, "self", Value::Unit(unit));
                    },
                    FnOwner::Instance(instance) => {
                        (*table_stack).define(&addr, "self", Value::Instance(instance));
                    }
                }
            }
            // вызов
            let native = (*function).function;
            native(self, addr.clone(), should_push, (*function).owner.clone())?;
            // успех
            Ok(())
        }
        else {
            error!(Error::new(
                addr.clone(),
                format!("{} is not a fn.", name),
                "you can call only fn-s.".to_string()
            ));
            Ok(())
        }
    }

    // загрузка значения переменной
    pub unsafe fn op_call(&mut self, addr: &Address, name: &str, has_previous: bool,
                                 should_push: bool, args: &Chunk) -> Result<(), ControlFlow> {
        // если нет предыдущего
        if !has_previous {
            // получаем значение
            let lookup_result = (*self.table_stack).find(&addr, &name);
            // проверяем на ошибку
            if let Err(e) = lookup_result {
                // ошибка
                error!(e)
            }
            else if let Ok(value) = lookup_result {
                // вызываем
                self.call(addr, &name, value, &args, should_push)?;
            }
        }
        // если есть
        else {
            // получаем значение
            let previous = self.pop(&addr)?;
            // проверяем
            match previous {
                Value::Instance(instance) => {
                    // получаем значение
                    let lookup_result = (*(*instance).fields).find(&addr, &name);
                    // проверяем на ошибку
                    if let Err(e) = lookup_result {
                        // ошибка
                        error!(e)
                    }
                    else if let Ok(value) = lookup_result {
                        // вызываем
                        self.call(addr, &name, value, args, should_push)?;
                    }
                }
                Value::Unit(unit) => {
                    // получаем значение
                    let lookup_result = (*(*unit).fields).find(&addr, &name);
                    // проверяем на ошибку
                    if let Err(e) = lookup_result {
                        // ошибка
                        error!(e)
                    }
                    else if let Ok(value) = lookup_result {
                        // вызываем
                        self.call(addr, &name, value, args, should_push)?;
                    }
                }
                _ => {
                    error!(Error::new(
                        addr.clone(),
                        format!("couldn't call {} from {:?}.", name, previous),
                        "you can call fn from unit, instance or foreign.".to_string()
                    ))
                }
            }
        }
        // успех
        Ok(())
    }

    // дублирование значения в стеке
    unsafe fn op_duplicate(&mut self, addr: &Address) -> Result<(), ControlFlow> {
        // операнд
        let operand = self.pop(&addr)?;
        // пушим
        self.push(operand);
        self.push(operand);
        // успех
        Ok(())
    }

    // проверка трейтов
    unsafe fn check_traits(&mut self, addr: &Address, instance: *mut Instance) {
        // тип инстанса
        let instance_type = (*instance).t;
        // получение трейта
        unsafe fn get_trait(traits: *mut Table, addr: &Address, trait_name: String) -> Option<*mut Trait> {
            // трейт
            let trait_result = (*traits).find(&addr, &trait_name);
            // проверяем результат
            if let Err(e) = trait_result {
                error!(e);
                None
            }
            else if let Ok(trait_value) = trait_result {
                match trait_value {
                    Value::Trait(_trait) => {
                        // перебираем функции
                        return Some(_trait)
                    }
                    _ => {
                        panic!("not a trait in traits table. report to developer.")
                    }
                }
            }
            else {
                return None
            }
        }
        // получение имплементации
        unsafe fn get_impl(table: *mut Table, addr: &Address, impl_name: String) -> Option<*mut Function> {
            // трейт
            let fn_result = (*table).find(&addr, &impl_name);
            // проверяем результат
            if let Err(e) = fn_result {
                error!(e);
                None
            }
            else if let Ok(trait_value) = fn_result {
                return match trait_value {
                    Value::Fn(_fn) => {
                        // перебираем функции
                        Some(_fn)
                    }
                    _ => {
                        None
                    }
                }
            }
            else {
                return None
            }
        }
        // проверка
        for trait_name in (*instance_type).impls.clone() {
            // получаем трейт
            let _trait = get_trait(self.traits, &addr, trait_name.clone()).unwrap();
            // проверяем
            for function in (*_trait).functions.clone() {
                // проверяем наличие имплементации
                if (*(*instance).fields).exists(&function.name) {
                    // имплементация
                    let _impl = get_impl((*instance).fields, addr, function.name.clone());
                    // проверяем
                    if _impl.is_some() {
                        // имплементация
                        let implementation = _impl.unwrap();
                        // проверяем имплементацию
                        if (*implementation).params.len() != function.params_amount {
                            // ошибка
                            error!(Error::new(
                                addr.clone(),
                                format!(
                                    "type {} impls {}, but fn {} has wrong impl.",
                                    (*instance_type).name.name,
                                    trait_name, function.name
                                ),
                                format!(
                                    "expected args {}, got {}",
                                    function.params_amount,
                                    (*implementation).params.len()
                                )
                            ));
                        }
                    }
                    else {
                        // ошибка
                        error!(Error::new(
                            addr.clone(),
                            format!(
                                "type {} impls {}, but doesn't impl fn {}({})",
                                (*instance_type).name.name.clone(),
                                trait_name, function.name.clone(),
                                function.params_amount
                            ),
                            format!("implement fn {}", function.name.clone())
                        ));
                    }
                }
                else {
                    // проверяем есть ли дефолтная имплементация
                    if function.default.is_some() {
                        // если есть
                        if let Err(e) = (*(*instance).fields).define(
                            &addr,
                            &function.name,
                            Value::Fn(memory::alloc_value(
                                function.default.unwrap(),
                            ))
                        ) {
                            error!(e);
                        }
                    }
                    // если нет
                    else {
                        // ошибка
                        error!(Error::new(
                            addr.clone(),
                            format!(
                                "type {} impls {}, but doesn't impl fn {}({})",
                                (*instance_type).name.name, // todo check
                                trait_name, function.name, // todo check
                                function.params_amount
                            ),
                            format!("implement fn {}", function.name)
                        ));
                    }
                }
            }
        }
    }

    // созедание экземпляра типа
    unsafe fn op_instance(&mut self, addr: &Address, name: &str,
                          args: &Chunk, should_push: bool) -> Result<(), ControlFlow> {

        // подгрузка конструктора
        unsafe fn pass_constructor(vm: &mut VM, addr: &Address, name: &str, params_amount: usize,
                                 args: &Chunk, params: Vec<String>, fields_table: *mut Table) -> Result<(), ControlFlow> {
            // фиксируем размер стека
            let prev_size = vm.evaluation_stack.len();
            // загрузка аргументов
            vm.run(args)?;
            // фиксируем новый размер стека
            let new_size = vm.evaluation_stack.len();
            // количество переданных аргументов
            let passed_amount = new_size-prev_size;
            // проверяем
            if passed_amount == params_amount {
                // проходимся по реверсированным параметрам
                for param in params.iter().rev() {
                    // получаем аргумент из стека
                    let operand = vm.pop(&addr)?;
                    // устанавливаем в таблице
                    if let Err(e) = (*fields_table).define(&addr, &param, operand) {
                        error!(e);
                    }
                }
                Ok(())
            } else {
                error!(Error::new(
                    addr.clone(),
                    format!("invalid args amount: {} to create instance of {}.", passed_amount, name),
                    format!("expected {} arguments.", params_amount)
                ));
                Ok(())
            }
        }
        // ищем тип
        let lookup_result = (*self.table_stack).find(&addr, &name);
        // проверяем, найден ли
        if let Ok(value) = lookup_result {
            // проверяем тип ли
            match value {
                Value::Type(t) => {
                    // создаём экземпляр
                    let instance = memory::alloc_value(Instance::new(
                        t,
                        memory::alloc_value(Table::new()),
                    ));
                    // добавляем в учет gc
                    self.gc_register(Value::Instance(instance));
                    // конструктор
                    pass_constructor(
                        self,
                        &addr,
                        name,
                        (*t).constructor.len(),
                        args,
                        (*t).constructor.clone(),
                        (*instance).fields
                    )?;
                    // временно пушим таблицу
                    let mut instance_frame = StackFrame::new(None);
                    instance_frame.table = (*instance).fields;
                    (*self.table_stack).append_frame(instance_frame);
                    // временный self
                    (*(*instance).fields).fields.insert("self".to_string(), Value::Instance(instance));
                    // исполняем тело
                    self.run(&*(*t).body)?;
                    // удаляем временный self
                    (*(*instance).fields).fields.remove(&"self".to_string());
                    // проверка трейтов
                    self.check_traits(addr, instance);
                    // бинды
                    self.bind_functions((*instance).fields, FnOwner::Instance(instance));
                    // удаляем таблицу из стека
                    (*self.table_stack).remove_frame();
                    // значение экземпляра
                    let instance_value = Value::Instance(
                        instance
                    );
                    // init функция
                    let init_fn = "init".to_string();
                    if (*(*instance).fields).exists(&init_fn) {
                        // пушим инстанс
                        self.push(instance_value);
                        // вызываем
                        self.op_call(
                            addr,
                            &init_fn,
                            true,
                            false,
                            &Chunk::new(vec![])
                        )?
                    }
                    // пушим
                    if should_push {
                        self.push(instance_value);
                    }
                    // успех
                    Ok(())
                }
                _ => {
                    panic!("found a non-type value in types table.")
                }
            }
        }
        else {
            error!(lookup_result.unwrap_err());
            Ok(())
        }
    }

    // окончание цикла
    #[allow(unused_variables)]
    unsafe fn op_endloop(&mut self, addr: &Address, current_iteration: bool) -> Result<(), ControlFlow> {
        if current_iteration {
            Err(ControlFlow::Continue)
        } else {
            Err(ControlFlow::Break)
        }
    }

    // создание замыкания
    unsafe fn op_make_closure(&mut self, addr: &Address, name: &str) -> Result<(), ControlFlow> {
        // ищем
        let lookup_result = (*self.table_stack).find(&addr, name);
        // проверяем, нашло ли
        if let Ok(value) = lookup_result {
            // проверяем, функция ли
            if let Value::Fn(function) = value {
                // устанавливаем замыкание
                let current_frame = (*self.table_stack).top_frame(addr); // todo: root closure
                let table_clone = memory::alloc_value(
                    (*current_frame.table).clone()
                );
                (*function).closure = table_clone;
                // успех
                Ok(())
            }
            else {
                // ошибка
                error!(Error::new(
                    addr.clone(),
                    format!("could not make closure for: {}", name),
                    "not a function.".to_string()
                ));
                Ok(())
            }
        }
        else {
            error!(
                lookup_result.unwrap_err()
            );
            Ok(())
        }
    }

    // возврат значения из функции
    unsafe fn op_return(&mut self, addr: &Address, value: &Chunk) -> Result<(), ControlFlow> {
        // выполняем
        self.run(value)?;
        let value = self.pop(&addr)?;
        // возвращаем
        Err(ControlFlow::Return(value))
    }

    // нативная функция
    unsafe fn op_native(&mut self, addr: &Address, name: &str) -> Result<(), ControlFlow> {
        // лукап
        let result = (*self.natives).find(&addr, &name);
        // если нашлась нативная функция
        if let Ok(value) = result {
            self.push(value);
        }
        // если нет
        if let Err(e) = result {
            error!(e);
        }
        // ок
        Ok(())
    }

    // "пробрасывание" ошибок
    unsafe fn op_error_propagation(&mut self, addr: &Address, value: &Chunk) -> Result<(), ControlFlow> {
        // выполняем
        self.run(value)?;
        // значение
        let value = self.pop(&addr)?;
        // вызов is_ok
        unsafe fn call_is_ok(vm: &mut VM, addr: &Address, instance: *mut Instance) -> Result<bool, ControlFlow> {
            // пробуем получить is_ok
            let lookup_result = (*(*instance).fields).find(&addr, "is_ok");
            // если успешно
            if let Ok(callable) = lookup_result {
                // проверяем, функция ли
                if let Value::Fn(function) = callable {
                    // проверяем количество аргументов
                    if (*function).params.len() != 0 {
                        error!(Error::new(
                            addr.clone(),
                            format!("is_ok takes {} params", (*function).params.len()),
                            "is_ok should take 0 params.".to_string()
                        ));
                        return Ok(false);
                    }
                }
                // если нет
                else {
                    error!(Error::new(
                            addr.clone(),
                            "is_ok is not a fn.".to_string(),
                            "is_ok should be fn.".to_string()
                        ));
                    return Ok(false);
                }
                // вызываем
                vm.call(
                    &addr, "is_ok", callable,
                    &Chunk::new(vec![]),
                    true
                )?;
                // получаем значение
                let is_ok = vm.pop(&addr)?;
                // проверяем, бул ли
                return if let Value::Bool(boolean) = is_ok {
                    Ok(boolean)
                } else {
                    error!(Error::new(
                        addr.clone(),
                        "is_ok should return a bool.".to_string(),
                        format!("it returned: {:?}", is_ok)
                    ));
                    Ok(false)
                }
            }
            // если ошибка
            else if let Err(e) = lookup_result {
                error!(e);
                return Ok(false)
            }
            // dead code
            Ok(false)
        }
        // вызов unwrap
        unsafe fn call_unwrap(vm: &mut VM, addr: &Address, instance: *mut Instance) -> Result<(), ControlFlow> {
            // пробуем получить is_ok
            let lookup_result = (*(*instance).fields).find(&addr, "unwrap");
            // если успешно
            match lookup_result {
                Ok(callable) => {
                    // проверяем, функция ли
                    if let Value::Fn(function) = callable {
                        // проверяем количество аргументов
                        if (*function).params.len() != 0 {
                            error!(Error::new(
                            addr.clone(),
                            format!("unwrap takes {} params", (*function).params.len()),
                            "unwrap should take 0 params.".to_string()
                        ));
                            return Ok(());
                        }
                    }
                    // если нет
                    else {
                        error!(Error::new(
                            addr.clone(),
                            "unwrap is not a fn.".to_string(),
                            "unwrap should be fn.".to_string()
                        ));
                        return Ok(());
                    }
                    // вызываем
                    vm.call(
                        &addr, "unwrap", callable,
                        &Chunk::new(vec![]),
                        true
                    )?;
                    // успех
                    Ok(())
                },
                Err(e) => {
                    // ошибка
                    error!(e);
                    // успех
                    Ok(())
                }
            }
        }
        // проверяем тип значения
        if let Value::Instance(instance) = value {
            // вызов is_ok
            let is_ok = call_is_ok(
                self,
                &addr,
                instance,
            )?;
            // проверяем is_ok
            // если есть ошибка
            if !is_ok {
                // возвращаем обратно
                return Err(
                    ControlFlow::Return(value)
                );
            }
            // если нет ошибки
            else {
                // вызываем unwrap
                call_unwrap(
                    self,
                    addr,
                    instance
                )?;
            }
        }
        // если неверный тип значения - ошибка
        else {
            error!(Error::new(
                addr.clone(),
                format!("could not use error propagation with {:?}.", value),
                "requires instance of type that impls .is_ok() and .unwrap() fn-s.".to_string()
            ))
        }
        // успех
        Ok(())
    }

    // проверка имплементации трейта
    unsafe fn op_impls(&mut self, addr: &Address, value: &Chunk, trait_name: &str) -> Result<(), ControlFlow> {
        // выполняем
        self.run(value)?;
        // значение
        let value = self.pop(&addr)?;
        // проверка, экземпляр ли класс значение
        if let Value::Instance(instance) = value {
            // ищем трейт
            let lookup_result = (*self.traits).find(&addr, &trait_name);
            // если нашли
            if let Ok(trait_value) = lookup_result {
                // проверяем, трейт ли
                match trait_value {
                    // если трейт
                    Value::Trait(_trait) => {
                        // список имплементаций
                        let impls = (*(*instance).t).impls.clone();
                        // имена трейта
                        let name = (*_trait).name.name.clone();
                        let full_name_option = (*_trait).name.full_name.clone();
                        // если есть полное имя
                        if let Some(full_name) = full_name_option {
                            // пушим бул, есть ли трейт в имплементациях
                            self.push(Value::Bool(
                                impls.contains(&name) || impls.contains(&full_name),
                            ));
                        }
                        // если нет
                        else {
                            // пушим бул, есть ли трейт в имплементациях
                            self.push(Value::Bool(
                                impls.contains(&name),
                            ));
                        }
                    }
                    // если нет
                    _ => {
                        panic!("not a trait in traits table. report to developer.")
                    }
                }
            }
            // если трейта не существует
            else if let Err(e) = lookup_result {
                error!(e);
            }
        }
        else {
            error!(Error::new(
                addr.clone(),
                format!("could not use impls with {:?}.", value),
                "impls op requires instance.".to_string()
            ))
        }
        // успех
        Ok(())
    }

    // удаление локальной переменной
    #[allow(unused_variables)]
    unsafe fn op_delete_local(&self, addr: &Address, name: &String) {
        (*self.table_stack).delete(name);
    }

    // запуск байткода
    #[allow(unused_variables)]
    pub unsafe fn run(&mut self, chunk: &Chunk) -> Result<(), ControlFlow> {
        for op in chunk.opcodes() {
            match op {
                Opcode::Push { addr, value } => {
                    self.op_push(value.clone())?;
                }
                Opcode::Pop { addr } => {
                    self.pop(&addr)?;
                }
                Opcode::Bin { addr, op } => {
                    self.op_binary(addr, &op)?;
                }
                Opcode::Neg { addr } => {
                    self.op_negate(addr)?;
                }
                Opcode::Bang { addr } => {
                    self.op_bang(&addr)?;
                }
                Opcode::Cond { addr, op } => {
                    self.op_conditional(&addr, &op)?;
                }
                Opcode::Logic { addr, op } => {
                    self.op_logical(addr, &op)?
                }
                Opcode::If { addr, cond, body, elif } => {
                    self.op_if(addr, cond, body, elif)?;
                }
                Opcode::Loop { addr, body } => {
                    self.op_loop(addr, body)?;
                }
                Opcode::DefineFn { addr, name, full_name, body, params } => {
                    self.op_define_fn(addr, &Symbol::new_option(name.clone(), full_name.clone()), body, params)?;
                }
                Opcode::DefineType { addr, name, full_name, body, constructor, impls } => {
                    self.op_define_type(addr, &Symbol::new_option(name.clone(), full_name.clone()), body, constructor, impls)?
                }
                Opcode::DefineUnit { addr, name, full_name, body } => {
                    self.op_define_unit(addr, &Symbol::new_option(name.clone(), full_name.clone()), body)?
                }
                Opcode::DefineTrait { addr, name, full_name, functions } => {
                    self.op_define_trait(addr, &Symbol::new_option(name.clone(), full_name.clone()), functions)?
                }
                Opcode::Define { addr, name, value, has_previous} => {
                    self.op_define(addr, name, *has_previous, value)?;
                }
                Opcode::Set { addr, name, value, has_previous } => {
                    self.op_set(addr, name, *has_previous, value)?;
                }
                Opcode::Load { addr, name, has_previous, should_push } => {
                    self.op_load(addr, name, *has_previous, *should_push)?;
                }
                Opcode::Call { addr, name, has_previous, should_push, args } => {
                    self.op_call(addr, name, *has_previous, *should_push, args)?
                }
                Opcode::Duplicate { addr } => {
                    self.op_duplicate(addr)?;
                }
                Opcode::Instance { addr, name, args, should_push } => {
                    self.op_instance(addr, name, args, *should_push)?;
                }
                Opcode::EndLoop { addr, current_iteration } => {
                    self.op_endloop(addr, *current_iteration)?;
                }
                Opcode::Closure { addr, name } => {
                    self.op_make_closure(addr, name)?;
                }
                Opcode::Ret { addr, value } => {
                    self.op_return(addr, value)?;
                }
                Opcode::Native { addr, fn_name } => {
                    self.op_native(addr, fn_name)?;
                }
                Opcode::ErrorPropagation { addr, value } => {
                    self.op_error_propagation(addr, value)?;
                }
                Opcode::Impls { addr, value, trait_name } => {
                    self.op_impls(addr, value, trait_name)?;
                }
                Opcode::DeleteLocal { addr, name } => {
                    self.op_delete_local(addr, name);
                }
            }
        }
        Ok(())
    }
}

// имплементация для передачи между потоками
unsafe impl Send for VM {}
unsafe impl Sync for VM {}