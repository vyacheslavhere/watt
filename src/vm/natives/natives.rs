﻿// импорты
use crate::errors::errors::Error;
use crate::lexer::address::Address;
use crate::vm::flow::ControlFlow;
use crate::vm::memory::memory;
use crate::vm::vm::{VM};
use crate::vm::natives::libs::*;
use crate::vm::table::Table;
use crate::vm::values::{FnOwner, Native, Symbol, Value};
use crate::error;

// провайд билтинов
pub unsafe fn provide_builtins(vm: &mut VM) -> Result<(), Error> {
    // билт-ин адрес
    let built_in_address: Address = Address::new(
        0,
        0,
        "-".to_string(),
        "-".to_string()
    );
    // io
    natives_base::provide(built_in_address.clone(), vm)?;
    natives_io::provide(built_in_address.clone(), vm)?;
    natives_list::provide(built_in_address.clone(), vm)?;
    natives_gc::provide(built_in_address.clone(), vm)?;
    // успех
    Ok(())
}

// провайд 1 функции
pub unsafe fn provide(
    vm: &mut VM,
    addr: Address,
    params_amount: usize,
    name: String,
    native: fn(&mut VM,Address,bool,*mut Table,*mut FnOwner) -> Result<(), ControlFlow>) {
    // дефайн
    if let Err(e) = (*vm.natives).define(
        addr,
        name.clone(),
        Value::Native(
            memory::alloc_value(
                Native::new(
                    Symbol::by_name(name),
                    params_amount,
                    native
                )
            )
        )
    ) {
        error!(e);
    }
}