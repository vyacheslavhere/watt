// imports
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

/// Runs code from a file
///
/// # Run args
///
/// * `gc_threshold`: garbage collector threshold
/// * `gc_debug`: on/off garbage collector debug
/// * `lexer_debug`: on/off lexer debug
/// * `ast_debug`: on/off ast debug
/// * `opcodes_debug`: on/of opcodes debug
/// * `lexer_bench`: on/off lexer benchmark
/// * `parser_bench`: on/off parser benchmark
/// * `compile_bench`: on/off compile benchmark
/// * `runtime_bench`: on/off runtime benchmark
///
#[allow(unused_qualifications)]
pub unsafe fn run(
    path: PathBuf,
    gc_threshold: Option<usize>,
    gc_debug: bool,
    lexer_debug: bool,
    ast_debug: bool,
    opcodes_debug: bool,
    lexer_bench: bool,
    parser_bench: bool,
    compile_bench: bool,
    runtime_bench: bool,
) {
    // reading file
    let code = read_file(Option::None, &path);

    // lexing
    let tokens = lex(
        &path,
        &code.chars().collect::<Vec<char>>(),
        lexer_debug,
        lexer_bench
    );

    // parsing
    let ast = parse(
        &path,
        tokens.unwrap(),
        ast_debug,
        parser_bench,
        &None
    );

    // analyzing
    let analyzed = analyze(
        ast.unwrap()
    );

    // compiling
    let compiled = compile(
        &analyzed,
        opcodes_debug,
        compile_bench
    );

    // run compiled opcodes chunk with vm
    run_chunk(
        compiled,
        gc_threshold.unwrap_or(200),
        gc_debug,
        runtime_bench
    );
}

/// Crashes program with text
pub fn crash(reason: String) -> ! {
    println!("{}", reason);
    std::process::exit(1);
}

/// Reading file
///
/// raises error if path is not exists,
/// or file can not be read.
///
pub fn read_file(addr: Option<Address>, path: &PathBuf) -> String {
    // If path already exist, pass it
    let path = if path.exists() {
        path
    } else {
        // If not, so we take the directory path of our program that imports the file
        let mut rpath = addr
            .as_ref()
            .and_then(|x| x.file.as_ref()).unwrap()
            .parent().map(|x| x.to_path_buf()).unwrap();

        // And add library name to it.
        rpath.push(path);

        &rpath.clone()
    };

    if path.exists() {
        if let Ok(result) = fs::read_to_string(path) {
            result
        } else {
            if let Some(address) = addr {
                error!(Error::own_text(
                        address,
                        format!("io error with file: {:?}", path),
                        "check file existence"
                    ));
            } else {
                crash(format!("file not found: {:?}", path));
            }
        }
    }
    else {
        if let Some(address) = addr {
            error!(Error::own_text(
                    address,
                    format!("file not found: {:?}", path),
                    "check file existence"
                ));
        } else {
            crash(format!("file not found: {:?}", path));
        }
    }
}

/// Lexing source code
/// Provides tokens on the exhaust
pub fn lex(file_path: &PathBuf, code: &[char], debug: bool, bench: bool) -> Option<Vec<Token>> {
    // benchmark
    let start = std::time::Instant::now();

    // lexing
    let tokens = Lexer::new(
        code,
        file_path
    ).lex();

    // benchmark end
    if bench {
        let duration = start.elapsed().as_nanos();
        println!("benchmark 'lexer', elapsed {}", duration as f64 / 1_000_000f64);
    }

    // debug
    if debug {
        println!("tokens debug: ");
        println!("{:?}", tokens);
    }

    Some(tokens)
}


/// Parsing
/// Provides AST node on the exhaust
pub fn parse(file_path: &PathBuf, tokens: Vec<Token>,
             debug: bool, bench: bool, full_name_prefix: &Option<String>) -> Option<Node> {
    // benchmark
    let start = std::time::Instant::now();

    // creating default full_name_prefix
    let file_name = file_path.file_name().and_then(|x| x.to_str()).unwrap();
    fn delete_extension(full_name: &str) -> &str {
        match full_name.rfind(".") {
            Some(index) => {
                &full_name[..index]
            }
            None => {
                full_name
            }
        }
    }

    // building ast
    let raw_ast = Parser::new(
        tokens,
        file_path,
        delete_extension(full_name_prefix.as_ref().map(String::as_str).unwrap_or(file_name))
    ).parse();

    // benchmark end
    if bench {
        let duration = start.elapsed().as_nanos();
        println!("benchmark 'parse', elapsed {}", duration as f64 / 1_000_000f64);
    }

    // handling errors
    if let Ok(ast) = raw_ast {
        // debug
        if debug {
            println!("ast debug: ");
            println!("{:?}", ast);
        }
        return Some(ast)
    } else if let Err(error) = raw_ast {
        error!(error);
    };

    // panic, if something went wrong
    panic!("result error in parsing. report to developer.")
}

/// Semantic analyzer
/// Provides analyzed node on the exhaust
pub fn analyze(ast: Node) -> Node {
    Analyzer::new().analyze(&ast);
    ast
}

/// Compilation
/// Provides compiled chunk on the exhaust
pub unsafe fn compile(ast: &Node, opcodes_debug: bool, bench: bool) -> Chunk {
    // benchmark
    let start = std::time::Instant::now();

    // compile
    let compiled = CompileVisitor::new().compile(ast);

    // benchmark end
    if bench {
        let duration = start.elapsed().as_nanos();
        println!("benchmark 'compile', elapsed {}", duration as f64 / 1_000_000f64);
    }

    // debug
    if opcodes_debug {
        println!("opcodes debug: ");
        for op in compiled.opcodes() {
            op.print(0);
        }
    }

    compiled
}

/// Runs chunk on the vm
/// 
/// * gc_threshold: garbage collector threshold
#[allow(unused_qualifications)]
unsafe fn run_chunk(chunk: Chunk, gc_threshold: usize, gc_debug: bool, bench: bool) {
    // benchmark
    let start = std::time::Instant::now();

    // creating vm and running
    let mut vm = VM::new(VmSettings::new(
        gc_threshold,
        gc_debug,
    ));
    
    // handling errors
    if let Err(e) = vm.run(&chunk, vm.globals) {
        error!(Error::own_text(
            Address::unknown(),
            format!("control flow leak: {e:?}"),
            "report this error to the developer."
        ));
    }

    // benchmark end
    if bench {
        let duration = start.elapsed().as_nanos();
        println!("benchmark 'runtime', elapsed {}", duration as f64 / 1_000_000f64);
    }

    // cleanup
    vm.cleanup();
}
