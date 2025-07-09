﻿// импорт
use crate::executor::executor;
use crate::parser::import::Import;
use core::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::lexer::address::Address;
use crate::parser::ast::Node;

// ресолвер импортов
pub struct ImportsResolver<'import_key, 'import_path> {
    imported: RefCell<Vec<String>>,
    libraries: HashMap<&'import_key str, &'import_path str>,
    builtins: Vec<String>
}
// имплементация
#[allow(unused_qualifications)]
impl<'import_key, 'import_path> ImportsResolver<'import_key, 'import_path> {
    // новый
    pub fn new() -> Self {
        ImportsResolver {
            imported: RefCell::new(vec![]),
            libraries: HashMap::from([
                ("std.io", "./libs/std/std_io.wt"),
                ("std.gc", "./libs/std/std_gc.wt"),
                ("std.errors", "./libs/std/std_errors.wt"),
                ("std.convert", "./libs/std/std_convert.wt"),
                ("std.typeof", "./libs/std/std_typeof.wt"),
                ("std.time", "./libs/std/std_time.wt"),
                ("std.math", "./libs/std/std_math.wt"),
                ("std.random", "./libs/std/std_random.wt"),
                ("std.fs", "./libs/std/std_fs.wt"),
                ("std.system", "./libs/std/std_system.wt"),
            ]),
            builtins: vec!["./libs/base.wt".to_string()],
        }
    }

    // импорт билт-инов
    pub fn import_builtins(&mut self) -> Vec<Node> {
        // ноды
        let mut nodes = vec![];
        // перебираем билт-ины
        for builtin in &self.builtins {
            if !self.imported.borrow().contains(&builtin) {
                // нода
                let node_option = self.import(
                    None,
                    &Import::new(None, builtin.to_string(), None)
                );
                // импортируем
                if let Some(node) = node_option {
                    nodes.push(node);
                }
            }
        }
        // возвращаем
        nodes
    }

    // ресолвинг
    fn resolve(
        &self,
        addr: Option<Address>,
        import: &Import
    ) -> Node {
        // ищем импорт
        let file: &str = if self.libraries.contains_key(import.name.as_str()) {
            self.libraries.get(import.name.as_str()).unwrap()
        } else {
            &import.name
        };
        // путь
        let path = PathBuf::from(file);
        // чтение файла
        let code = executor::read_file(addr, &path);
        // имя файла
        let filename = path.file_name().unwrap().to_str().unwrap();
        // компиляция
        let tokens = executor::lex(
            filename,
            &code.chars().collect::<Vec<char>>(),
            false,
            false
        );
        let ast = executor::parse(
            filename,
            tokens.unwrap(),
            false,
            false,
            &import.full_name
        );
        let mut analyzed = executor::analyze(
            ast.unwrap()
        );
        // блок результата
        let result: Node;
        // проверяем блок
        if let Node::Block { body } = &mut analyzed {
            // новое тело
            let mut new_body: Vec<Node> = vec![];
            // добавляем в тело
            for node in body.drain(..) {
                // перебираем
                match node {
                    Node::Native { .. } |
                    Node::FnDeclaration { .. } |
                    Node::Type { .. } |
                    Node::Unit { .. } |
                    Node::Trait { .. } | 
                    Node::Import { .. } => {
                        new_body.push(node);
                    }
                    _ => {}
                }
            }
            // результат
            result = Node::Block { body: new_body };
        }
        // в случае ошибки
        else {
            // ошибка
            panic!("parser returned non-block node as result. report to the developer.");
        }
        // возвращаем
        result
    }

    // импорт
    pub fn import(
        &self,
        addr: Option<Address>,
        import: &Import
    ) -> Option<Node> {
        // проверка на наличие импорта, если его нет
        if !self.imported.borrow().contains(&import.name) {
            // ресолвинг
            let node = self.resolve(addr, import);
            // импротируем
            self.imported.borrow_mut().push(import.name.clone());
            // возвращаем
            Option::Some(node)
        }
        // ничего
        else {
            Option::None
        }
    }
}
