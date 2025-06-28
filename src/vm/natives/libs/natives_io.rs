// импорты
use std::io::{self, Write};
use crate::error;
use crate::errors::errors::Error;
use crate::lexer::address::Address;
use crate::vm::bytecode::OpcodeValue;
use crate::vm::natives::natives;
use crate::vm::table::Table;
use crate::vm::values::{FnOwner, Value};
use crate::vm::vm::VM;

// провайд
#[allow(unused_variables)]
pub unsafe fn provide(built_in_address: Address, vm: &mut VM) -> Result<(), Error> {
    // функции
    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "io@println".to_string(),
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table, owner: Option<FnOwner>| {
            println!("{}", vm.pop(&addr)?);

            if should_push {
                vm.push(Value::Null)
            }
            
            Ok(())
        }
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "io@print".to_string(),
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table, owner: Option<FnOwner>| {
            print!("{}", vm.pop(&addr)?);

            if should_push {
                vm.push(Value::Null)
            }

            Ok(())
        }
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "io@flush".to_string(),
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table, owner: Option<FnOwner>| {
            io::stdout().lock().flush().unwrap();

            if should_push {
                vm.push(Value::Null)
            }
            
            Ok(())
        }
    );
    natives::provide(
        vm,
        built_in_address,
        0,
        "io@input".to_string(),
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table, owner: Option<FnOwner>| {
            // инпут
            let mut input: String = String::new();
            if let Err(e) = io::stdin()
                .read_line(&mut input) {
                error!(Error::new(
                    addr,
                    format!("io error in input: {}", e),
                    "check your code".to_string()
                ))
            }
            // если нужен пуш
            if should_push {
                vm.op_push(
                    OpcodeValue::String(
                        input
                    ),
                    table
                )?;
            }
            // успех
            Ok(())
        }
    );
    // успех
    Ok(())
}