// импорты
use crate::errors::errors::{Error};
use crate::lexer::address::Address;
use crate::vm::values::Value;
use std::collections::HashMap;
use crate::vm::memory::memory;

// таблица
#[derive(Clone, Debug)]
pub struct Table {
    pub fields: HashMap<String,Value>,
}
// имплементация
impl Table {
    // новая таблица
    pub fn new() -> Table {
        Table {
            fields: HashMap::new(),
        }
    }
    // проверка, есть ли значение
    pub unsafe fn exists(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    // поиск переменной
    pub unsafe fn find(&self, address: &Address, name: &str) -> Result<Value, Error> {
        if self.exists(name) {
            Ok(self.fields[name].clone())
        } else {
            Err(Error::new(
                address.clone(),
                format!("{name} is not found."),
                "check variable existence.".to_string()
            ))
        }
    }

    // декларация переменной
    pub fn define(&mut self, address: &Address, name: &str, value: Value) -> Result<(), Error> {
        if !self.fields.contains_key(name) {
            self.fields.insert(name.to_string(), value);
            Ok(())
        } else {
            Err(Error::new(
                address.clone(),
                format!("{name} is already defined."),
                "you can rename variable.".to_string()
            ))
        }
    }

    // установка значения переменной
    pub unsafe fn set(&mut self, address: &Address, name: &str, value: Value) -> Result<(), Error> {
        if self.fields.contains_key(name) {
            self.fields.insert(name.to_string(), value);
            Ok(())
        } else {
            Err(Error::new(
                address.clone(),
                format!("{name} is not defined."),
                "you can define it, using := op.".to_string()
            ))
        }
    }

    // глубокая очистка
    pub unsafe fn free(&self) {
        let mut to_free = vec![];
        for v in self.fields.values() {
            if !to_free.contains(v) {
                to_free.push(v.clone());
            }
        }
        for val in to_free {
            match val {
                Value::Fn(f) => {
                    if !f.is_null() { memory::free_value(f); }
                }
                Value::Instance(i) => {
                    if !i.is_null() { memory::free_value(i); }
                }
                Value::String(s) => {
                    if !s.is_null() { memory::free_const_value(s); }
                }
                Value::Native(n) => {
                    if !n.is_null() { memory::free_value(n); }
                }
                Value::Unit(u) => {
                    if !u.is_null() { memory::free_value(u); }
                }
                Value::List(l) => {
                    if !l.is_null() { memory::free_value(l); }
                }
                Value::Type(t) => {
                    if !t.is_null() { memory::free_value(t); }
                }
                Value::Trait(t) => {
                    if !t.is_null() { memory::free_value(t); }
                }                
                _ => {}
            }
        }
    }
}

// имплементация Send и Sync для трансфера между потоками
unsafe impl Send for Table {}
unsafe impl Sync for Table {}