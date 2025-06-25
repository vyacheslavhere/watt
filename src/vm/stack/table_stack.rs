// импорты
use crate::vm::stack::table::Table;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use crate::error;
use crate::errors::errors::Error;
use crate::lexer::address::Address;
use crate::vm::memory::memory;
use crate::vm::values::Value;

// стак фрейм
#[derive(Clone)]
pub struct StackFrame {
    // таблица
    pub table: *mut Table,
    // замыкание
    pub closure: *mut Table
}
// имплементация
impl StackFrame {
    pub fn new(closure: Option<*mut Table>) -> Self {
        StackFrame {
            table: memory::alloc_value(Table::new()),
            closure: closure.unwrap_or(std::ptr::null_mut())
        }
    }
}
// имплементация debug
impl Debug for StackFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "({:?}, closure:{:?})", *self.table, self.closure) }
    }
}

// стэк таблиц
pub struct TableStack {
    pub stack: VecDeque<StackFrame>,
}

// имплемнтация
impl TableStack {
    // новый стэк
    pub fn new() -> Self {
        TableStack {
            stack: VecDeque::new()
        }
    }

    // удаление переменной
    pub unsafe fn delete(&self, name: &str) {
        (*self.stack.back().unwrap().table).fields.remove(name);
    }

    // существует ли переменная
    pub unsafe fn exists(&self, name: &str) -> bool {
        // если найдено
        for frame in self.stack.iter().rev() {
            if (*frame.table).exists(name) {
                return true;
            }
            if !frame.closure.is_null() && (*frame.closure).exists(name) {
                return true;
            }
        }
        // если не найдено
        false
    }

    // поиск переменной
    pub unsafe fn find(&self, addr: &Address, name: &str) -> Result<Value, Error> {
        // если найдено
        for frame in self.stack.iter().rev() {
            if (*frame.table).exists(name) {
                return (*frame.table).find(addr, name);
            }
            if !frame.closure.is_null() && (*frame.closure).exists(name) {
                return (*frame.closure).find(addr, name);
            }
        }
        // если не найдено
        Err(Error::new(
            addr.clone(),
            format!("not found: {}", name),
            "check variable existence".to_string()
        ))
    }

    // дефайн переменной
    pub unsafe fn define(&mut self, addr: &Address, name: &str, value: Value) {
        for frame in self.stack.iter().rev() {
            match (*frame.table).define(addr, name, value) {
                Ok(_) => return,
                Err(e) => error!(e),
            }
        }
    }

    // установка переменной
    pub unsafe fn set(&mut self, addr: &Address, name: &str, value: Value) {
        for frame in self.stack.iter().rev() {
            if (*frame.table).exists(name) {
                match (*frame.table).set(addr, name, value) {
                    Ok(_) => return,
                    Err(e) => error!(e),
                }
            }
        }
        error!(Error::new(
            addr.clone(),
            format!("{name} is not defined."),
            "you can define it, using := op.".to_string()
        ))
    }

    // пуш фрейма
    pub unsafe fn push_frame(&mut self, closure: Option<*mut Table>) {
        self.stack.push_back(StackFrame::new(
            closure
        ));
    }

    // добавление фрейм
    pub fn append_frame(&mut self, stack_frame: StackFrame) {
        self.stack.push_back(stack_frame);
    }

    // удаление фрейма
    pub fn remove_frame(&mut self) {
        self.stack.pop_back();
    }

    // поп фрейм
    pub unsafe fn pop_frame(&mut self, addr: &Address) {
        // проверка на underflow
        if self.stack.len() == 0 {
            error!(Error::new(
                addr.clone(),
                "table stack underflow.".to_string(),
                "could not pop a frame!".to_string()
            ))
        }
        // фрэйм
        let frame
            = self.stack.pop_back().unwrap();
        // очистка таблицы
        memory::free_value(frame.table);
        // очистка замыкания
        if !frame.closure.is_null() {
            memory::free_value(frame.closure);
        }
    }

    // top фрейм
    pub unsafe fn top_frame(&self, addr: &Address) -> StackFrame {
        // проверка на underframe
        if self.stack.len() == 0 {
            error!(Error::new(
                addr.clone(),
                "table stack underflow.".to_string(),
                "could not get top frame!".to_string()
            ))
        }
        // возвращам
        self.stack.back().unwrap().clone()
    }

    // очистка
    pub unsafe fn free(&mut self) {
        /*
        for frame in self.stack.iter().rev() { // TODO: freed native
            (*frame.table).free();
        }
         */
    }
}