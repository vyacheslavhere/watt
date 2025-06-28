// импорты
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use crate::lexer::address::Address;
use crate::vm::bytecode::Chunk;
use crate::vm::flow::ControlFlow;
use crate::vm::memory::memory;
use crate::vm::table::Table;
use crate::vm::vm::VM;

// символ
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub full_name: Option<String>,
}
impl Symbol {
    #[allow(unused_qualifications)]
    #[allow(unused)]
    pub fn new(name: String, full_name: String) -> Symbol {
        Symbol {name, full_name: Option::Some(full_name)}
    }
    pub fn new_option(name: String, full_name: Option<String>) -> Symbol {
        Symbol {name, full_name }
    }
    #[allow(unused_qualifications)]
    #[allow(unused)]
    pub fn by_name(name: String) -> Symbol {
        Symbol {name, full_name: Option::None}
    }
}

// тип
#[derive(Clone, Debug, Eq, PartialEq)] // todo check eq, partialeq
pub struct Type {
    pub name: Symbol,
    pub constructor: Vec<String>,
    pub body: *const Chunk,
    pub impls: Vec<String>
}
// имплементация
impl Type {
    pub fn new(name: Symbol, constructor: Vec<String>, body: *const Chunk, impls: Vec<String>) -> Type {
        Type {name, constructor, body, impls}
    }
}
// имплементация дропа
impl Drop for Type {
    fn drop(&mut self) {
        memory::free_const_value(self.body);
    }
}

// экземпляр типа
#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Instance {
    pub t: *mut Type,
    pub fields: *mut Table
}
// имплементация
impl Instance {
    pub fn new(t: *mut Type, fields: *mut Table) -> Instance {
        Instance {t, fields}
    }
}
// имплементация дропа
impl Drop for Instance {
    fn drop(&mut self) {
        memory::free_value(self.fields);
    }
}

// юнит
#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Unit {
    pub name: Symbol,
    pub fields: *mut Table,
}
impl Unit {
    pub fn new(name: Symbol, fields: *mut Table) -> Unit {
        Unit {name, fields}
    }
}
// имплементация дропа
impl Drop for Unit {
    fn drop(&mut self) {
        memory::free_value(self.fields);
    }
}

// функция трейта
#[derive(Clone, Debug)]
pub struct TraitFn {
    pub name: String,
    pub params_amount: usize,
    pub default: Option<Function>
}
impl TraitFn {
    pub fn new(name: String, params_amount: usize, default: Option<Function>) -> TraitFn {
        TraitFn {name, params_amount, default}
    }
}

// трейт
#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Trait {
    pub name: Symbol,
    pub functions: Vec<TraitFn>
}
impl Trait {
    pub fn new(name: Symbol, functions: Vec<TraitFn>) -> Trait {
        Trait {name, functions}
    }
}

// владелец функции
#[derive(Clone, Debug)]
pub enum FnOwner {
    Unit(*mut Unit),
    Instance(*mut Instance),
}

// функция
#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Function {
    pub name: Symbol,
    pub body: *const Chunk,
    pub params: Vec<String>,
    pub owner: Option<FnOwner>,
    pub closure: *mut Table
}
// имплементация
impl Function {
    pub fn new(name: Symbol, body: *const Chunk, params: Vec<String>) -> Function {
        Function {
            name,
            body,
            params,
            owner: None,
            closure: std::ptr::null_mut()
        }
    }
}
// имплементация дропа
impl Drop for Function {
    fn drop(&mut self) {
        memory::free_value(self.closure);
        memory::free_const_value(self.body);
    }
}

// нативная функция
#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Native {
    pub name: Symbol,
    pub params_amount: usize,
    pub function: fn(&mut VM,Address,bool,*mut Table,Option<FnOwner>) -> Result<(), ControlFlow>,
    pub owner: Option<FnOwner>,
}
impl Native {
    // новый
    pub fn new(
        name: Symbol,
        params_amount: usize,
        function: fn(&mut VM,Address,bool,*mut Table,Option<FnOwner>
        ) -> Result<(), ControlFlow>) -> Native {
        // возвращаем
        Native {
            name,
            params_amount,
            function,
            owner: None,
        }
    }
}

// значение
#[derive(Clone, Copy)]
pub enum Value {
    Float(f64),
    Int(i64),
    String(*const String),
    Bool(bool),
    Type(*mut Type),
    Fn(*mut Function),
    Native(*mut Native),
    Instance(*mut Instance),
    Unit(*mut Unit),
    Trait(*mut Trait),
    List(*mut Vec<Value>),
    Null
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            match self {
                Value::String(s) => {
                    write!(f, "{}", **s)
                },
                value => {
                    // Все остальные типы выводим как объекты.
                    write!(f, "{:?}", value)
                }
            }
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            match self {
                Value::String(s) => {
                    write!(f, "\"{}\"", **s)
                },
                Value::Instance(i) => {
                    write!(f, "Instance{:?}", *i)
                },
                Value::Trait(t) => {
                    write!(f, "Trait{:?}", *t)
                }
                Value::Fn(fun) => {
                    write!(f, "Fn{:?}", *fun)
                },
                Value::Native(n) => {
                    write!(f, "Native{:?}", *n)
                },
                Value::Unit(n) => {
                    write!(f, "Unit{:?}", *n)
                },
                Value::Null => {
                    write!(f, "Null")
                },
                Value::Bool(b) => {
                    write!(f, "{}", *b)
                }
                Value::Type(t) => {
                    write!(f, "Type{:?}", *t)
                }
                Value::Int(i) => {
                    write!(f, "{}", *i)
                }
                Value::Float(fl) => {
                    write!(f, "{}", *fl)
                }
                Value::List(l) => {
                    write!(f, "List{:?}", *l)
                }
            }
        }
    }
}

#[allow(unused_unsafe)]
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (*self, *other) {
            (Value::Instance(a), Value::Instance(b)) => unsafe {
                a == b
            }
            (Value::Fn(a), Value::Fn(b)) => unsafe {
                a == b
            }
            (Value::Native(a), Value::Native(b)) => unsafe {
                a == b
            }
            (Value::Bool(a), Value::Bool(b)) => unsafe {
                a == b
            }
            (Value::Type(a), Value::Type(b)) => unsafe {
                a == b
            }
            (Value::String(a), Value::String(b)) => unsafe {
                a == b
            }
            (Value::Int(a), Value::Int(b)) => {
                a == b
            }
            (Value::Float(a), Value::Float(b)) => {
                a == b
            }
            (Value::Unit(a), Value::Unit(b)) => unsafe {
                a == b
            }
            (Value::Trait(a), Value::Trait(b)) => unsafe {
                a == b
            }
            (Value::List(a), Value::List(b)) => unsafe {
                a == b
            }
            _ => false
        }
    }
}
impl Eq for Value {}
impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            Value::Instance(a) => {
                (a as usize).hash(state);
            }
            Value::Fn(a) => {
                (a as usize).hash(state);
            }
            Value::Native(a) => {
                (a as usize).hash(state);
            }
            Value::Bool(a) => {
                a.hash(state);
            }
            Value::Type(a) => {
                (a as usize).hash(state);
            }
            Value::String(a) => {
                (a as usize).hash(state);
            }
            Value::Int(a) => {
                a.hash(state);
            }
            Value::Float(a) => {
                a.to_bits().hash(state);
            }
            Value::Unit(a) => {
                (a as usize).hash(state);
            }
            Value::Trait(a) => {
                (a as usize).hash(state);
            }
            Value::List(a) => {
                (a as usize).hash(state);
            }
            Value::Null => {
                0.hash(state);
            }
        }
    }
}