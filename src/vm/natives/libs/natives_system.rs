// импорты
use crate::error;
use crate::errors::errors::Error;
use crate::lexer::address::Address;
use crate::vm::bytecode::OpcodeValue;
use crate::vm::memory::memory::{self};
use crate::vm::natives::libs::utils;
use crate::vm::natives::natives;
use crate::vm::table::Table;
use crate::vm::values::Value;
use crate::vm::vm::VM;
use std::process::Command;
use sysinfo::System;

// провайд
#[allow(unused_variables)]
pub unsafe fn provide(built_in_address: &Address, vm: &mut VM) -> Result<(), Error> {
    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "system@getenv",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если не надо пушить
            if !should_push {
                return Ok(());
            }

            // получаем значение
            let env_key = &*utils::expect_string(addr.clone(), vm.pop(&addr)?, None);
            let value = match std::env::vars().find(|x| &x.0 == env_key) {
                Some((key, value)) => {
                    value
                }
                None => {
                    vm.push(Value::Null);
                    return Ok(());
                }
            };

            vm.op_push(OpcodeValue::String(value), table)?;

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        2,
        "system@setenv",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // значение и ключ
            let env_value = &*utils::expect_string(addr.clone(), vm.pop(&addr)?, None);
            let env_key = &*utils::expect_string(addr.clone(), vm.pop(&addr)?, None);

            // установка значения
            std::env::set_var(env_key, env_value);

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "system@getcwd",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если не надо пушить
            if !should_push {
                return Ok(());
            }

            // получаем current dir
            let cwd = match std::env::current_dir() {
                Ok(cwd) => {
                    let path = cwd.to_str().map(|x| x.to_string());
                    match path {
                        Some(p) => {
                            p
                        }
                        None => {
                            vm.push(Value::Null);
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    vm.push(Value::Null);
                    return Ok(());
                }
            };

            vm.op_push(OpcodeValue::String(cwd), table)?;

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "system@getargs",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если не надо пушить
            if !should_push {
                return Ok(());
            }

            // аргументы
            let args: Vec<Value> = std::env::args()
                .skip(1)
                .map(|x| {
                    let string = Value::String(memory::alloc_value(x));

                    // I don't know how to register those values properly.
                    
                    vm.gc_guard(string);
                    vm.gc_register(string, table);

                    string
                })
                .collect();

            let raw_list = Value::List(memory::alloc_value(args));

            // пушим
            vm.op_push(OpcodeValue::Raw(raw_list), table)?;

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "system@memory_total",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если не надо пушить
            if !should_push {
                return Ok(());
            }
            // информация о памяти
            let system_info = System::new_with_specifics(
                sysinfo::RefreshKind::nothing()
                    .with_memory(sysinfo::MemoryRefreshKind::everything()),
            );
            vm.push(Value::Int(system_info.total_memory() as _));

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "system@memory_used",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если не надо пушить
            if !should_push {
                return Ok(());
            }

            // информация о памяти
            let system_info = System::new_with_specifics(
                sysinfo::RefreshKind::nothing()
                    .with_memory(sysinfo::MemoryRefreshKind::everything()),
            );

            vm.push(Value::Int(system_info.used_memory() as _));

            // успех
            Ok(())
        },
    );

    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "system@cpu_count",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если не надо пушить
            if !should_push {
                return Ok(());
            }

            // информация
            vm.push(Value::Int(
                std::thread::available_parallelism()
                    .map(|x| x.get())
                    .unwrap_or(1) as _,
            ));

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "system@this_process_id",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            vm.push(Value::Int(std::process::id() as _));
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "system@this_process_terminate",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            let code = utils::expect_int(addr.clone(), vm.pop(&addr)?, None);
            std::process::exit(code as _);
        },
    );

    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "system@process_spawn_shell",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если не надо пушить
            if !should_push {
                error!(Error::new(
                    addr.clone(),
                    "A value must be taken.",
                    "Give it a name: `process = std.process.spawn(...)`"
                ));
            }

            // формирование команды
            let command = &*utils::expect_string(addr.clone(), vm.pop(&addr)?, None);
            let mut descriptor = if cfg!(target_os = "windows") {
                let mut shell = Command::new("cmd");
                shell.args(["/C", command]);
                shell
            } else {
                let mut shell = Command::new("sh");
                shell.arg("-c").arg(command);
                shell
            };

            // исполнение
            match descriptor.spawn() {
                Ok(child) => {
                    vm.op_push(OpcodeValue::Raw(Value::Any(memory::alloc_value(child))), table)?;
                }
                Err(_e) => {
                    vm.push(Value::Null);
                }
            };

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "system@process_wait",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // процесс
            let child = &mut *utils::expect_any(addr.clone(), vm.pop(&addr)?, None);
            let child: Option<&mut std::process::Child> = child.downcast_mut();

            // ожидание
            match child {
                Some(ch) => {
                    let value = ch.wait();
                    if should_push {
                        match value {
                            Ok(status) => {
                                vm.push(Value::Int(status.code().unwrap_or(0) as _));
                            }
                            Err(e) => {
                                vm.push(Value::Null);
                            }
                        }
                    }
                }
                None => {
                    error!(Error::new(
                        addr.clone(),
                        "the inner raw value is not a `std::process::Child`",
                        "please file an issue at https://github.com/vyacheslavhere/watt"
                    ))
                }
            }

            // успех
            Ok(())
        },
    );

    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "system@process_terminate",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // процесс
            let child = &mut *utils::expect_any(addr.clone(), vm.pop(&addr)?, None);
            let child: Option<&mut std::process::Child> = child.downcast_mut();

            // убиваем процесс
            match child {
                Some(ch) => {
                    let _ = ch.kill();
                }
                None => {
                    error!(Error::new(
                        addr.clone(),
                        "The inner raw value is not a `std::process::Child`",
                        "please file an issue at https://github.com/vyacheslavhere/watt"
                    ))
                }
            }

            // если нужно пушить
            if should_push {
                vm.push(Value::Null);
            }

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        1,
        "system@process_id",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // процесс
            let child = &mut *utils::expect_any(addr.clone(), vm.pop(&addr)?, None);
            let child: Option<&mut std::process::Child> = child.downcast_mut();

            // получаем айди
            match child {
                Some(ch) => {
                    if should_push {
                        vm.push(Value::Int(ch.id() as _));
                    }
                }
                None => {
                    error!(Error::new(
                        addr.clone(),
                        "The inner raw value is not a `std::process::Child`",
                        "please file an issue at https://github.com/vyacheslavhere/watt"
                    ))
                }
            }

            // успех
            Ok(())
        },
    );
    natives::provide(
        vm,
        built_in_address.clone(),
        0,
        "system@get_osname",
        |vm: &mut VM, addr: Address, should_push: bool, table: *mut Table| {
            // если надо пушить
            if should_push {
                vm.push(Value::String(memory::alloc_value(std::env::consts::OS.to_string())));
            }

            // успех
            Ok(())
        },
    );
    // успех
    Ok(())
}
