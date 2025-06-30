// импорты
use std::fs;
use std::path::PathBuf;
use crate::compiler::visitor::CompileVisitor;
use crate::error;
use crate::errors::errors::Error;
use crate::lexer::address::Address;
use crate::lexer::lexer::{Lexer, Token};
use crate::parser::ast::Node;
use crate::parser::parser::Parser;
use crate::semantic::analyzer::Analyzer;
use crate::vm::bytecode::Chunk;
use crate::vm::vm::{VmSettings, VM};

// запуск кода
#[allow(unused_qualifications)]
pub unsafe fn run(
    path: PathBuf,
    gc_threshold: Option<usize>,
    gc_debug: Option<bool>,
    lexer_debug: Option<bool>,
    ast_debug: Option<bool>,
    opcodes_debug: Option<bool>,
    lexer_bench: Option<bool>,
    parser_bench: Option<bool>,
    compile_bench: Option<bool>,
    runtime_bench: Option<bool>,
) {
    // чтение файла
    let code = read_file(Option::None, &path);
    // имя файла
    let filename = path.file_name().unwrap().to_str().unwrap();
    // компиляция
    let tokens = lex(
        filename,
        &code,
        lexer_debug.unwrap_or(false),
        lexer_bench.unwrap_or(false)
    );
    let ast = parse(
        filename,
        tokens.unwrap(),
        ast_debug.unwrap_or(false),
        parser_bench.unwrap_or(false),
        &None
    );
    let analyzed = analyze(
        ast.unwrap()
    );
    let compiled = compile(
        &analyzed,
        opcodes_debug.unwrap_or(false),
        compile_bench.unwrap_or(false)
    );
    // запуск
    run_chunk(
        compiled,
        gc_threshold.unwrap_or(200),
        gc_debug.unwrap_or(false),
        runtime_bench.unwrap_or(false)
    );
}

// краш
pub fn crash(reason: String) {
    // крашим и выходим
    println!("{}", reason);
    std::process::exit(1);
}

// чтение файла
pub fn read_file(addr: Option<Address>, path: &PathBuf) -> String {
    // проверяем наличие пути, если есть
    if path.exists() {
        if let Ok(result) = fs::read_to_string(path) {
            result
        } else {
            if let Some(address) = addr {
                error!(Error::new(
                        address,
                        format!("io error with file: {:?}", path),
                        "check file existence".to_string()
                    ));
            } else {
                crash(
                    format!(
                        "file not found: {:?}",
                        path
                    )
                );
            }
            String::new()
        }
    }
    // если нет
    else {
        if let Some(address) = addr {
            error!(Error::new(
                    address,
                    format!("file not found: {:?}", path),
                    "check file existence".to_string()
                ));
        } else {
            crash(
                format!(
                    "file not found: {:?}",
                    path
                )
            );
        }
        "".to_string()
    }
}

// лексинг
pub fn lex(file_name: &str, code: &str, debug: bool, bench: bool) -> Option<Vec<Token>> {
    // начальное время
    let start = std::time::Instant::now();
    // сканнинг токенов
    let mut lexer = Lexer::new(code, file_name);
    lexer.lex();

    let tokens = lexer.tokens;
    // конечное время
    let duration = start.elapsed().as_nanos();
    if bench { println!("benchmark 'lexer', elapsed {}", duration as f64 / 1_000_000f64); }
    // проверяем на дебаг
    if debug {
        println!("tokens debug: ");
        println!("{:?}", tokens);
    }
    // возвращаем
    Some(tokens)
}


// парсинг
pub fn parse(file_name: &str, tokens: Vec<Token>, 
        debug: bool, bench: bool, full_name_prefix: &Option<String>) -> Option<Node> {
    // начальное время
    let start = std::time::Instant::now();
    // удаление расширения файла
    fn delete_extension(full_name: String) -> String {
        match full_name.rfind(".") {
            Some(index) => {
                full_name[..index].to_string()
            }
            None => {
                full_name
            }
        }
    }
    // стройка аст
    let raw_ast = Parser::new(
        tokens,
        file_name,
        delete_extension(full_name_prefix.as_ref().map(String::as_str).unwrap_or(file_name).to_string())
    ).parse();
    // конечное время
    let duration = start.elapsed().as_nanos();
    if bench { println!("benchmark 'parse', elapsed {}", duration as f64 / 1_000_000f64); }
    // проверяем на ошибку
    if let Ok(ast) = raw_ast {
        // проверяем на дебаг
        if debug {
            println!("ast debug: ");
            println!("{:?}", ast);
        }
        // возвращаем
        return Some(ast)
    } else if let Err(error) = raw_ast {
        // ошибка
        error!(error);
        // ничего не возвращаем
        return None
    };
    // паника
    panic!("result error in parsing. report to developer.")
}

// семантический анализ
pub fn analyze(ast: Node) -> Node {
    // анализ
    Analyzer::new().analyze(&ast);
    // возвращаем оригинальное дерево
    ast
}

// компиляция
pub unsafe fn compile(ast: &Node, opcodes_debug: bool, bench: bool) -> Chunk {
    // начальное время
    let start = std::time::Instant::now();
    // компилируем
    let compiled = CompileVisitor::new().compile(ast);
    // конечное время
    if bench {
        let duration = start.elapsed().as_nanos();
        
        println!("benchmark 'compile', elapsed {}", duration as f64 / 1_000_000f64);
    }
    // дебаг
    if opcodes_debug {
        println!("opcodes debug: ");
        println!("{:?}", &compiled);
    }
    // возвращаем
    compiled
}

// запуск
#[allow(unused_qualifications)]
unsafe fn run_chunk(chunk: Chunk, gc_threshold: usize, gc_debug: bool, bench: bool) {
    // начальное время
    let start = std::time::Instant::now();
    // вм
    let mut vm = VM::new(VmSettings::new(
        gc_threshold,
        gc_debug,
    ));
    // запуск
    if let Err(e) = vm.run(&chunk, vm.globals) {
        error!(Error::new(
            Address::unknown(),
            format!("control flow leak: {:?}", e),
            "report this error to the developer.".to_string()
        ));
    }
    // конечное время
    if bench {
        let duration = start.elapsed().as_nanos();
        println!("benchmark 'runtime', elapsed {}", duration as f64 / 1_000_000f64);
    }
    // высвобождаем таблицы и gc
    vm.cleanup();
}
