// импорты
use crate::errors::errors::Error;
use crate::lexer::address::Address;
use crate::vm::natives::natives;
use crate::vm::values::{FnOwner, Value};
use crate::vm::vm::VM;

// провайд
#[allow(unused_variables)]
pub unsafe fn provide(built_in_address: Address, vm: &mut VM) -> Result<(), Error> {
    // функции
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "gc@invoke".to_string(),
        |vm: &mut VM, addr: Address, should_push: bool, owner: Option<FnOwner>| {
            vm.gc_invoke();
            if should_push {
                vm.push(Value::Null)
            }
            Ok(())
        }
    );
    // успех
    Ok(())
}