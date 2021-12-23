mod codegen;
mod parser;
mod tokens;

use logos::Logos;

extern crate inkwell;

use std::borrow::Borrow;
use std::io::{Read};
use clap::{App, Arg};
use inkwell::context::Context;
use inkwell::OptimizationLevel;

use crate::codegen::Compiler;
use crate::tokens::Token;
use crate::parser::Parser;


fn main() {

    let matches = App::new("MeidoLang")
        .version("0.1.0")
        .arg(Arg::with_name("jit")
            .short("j")
            .long("jit")
            .help("Specifies to run with just in time compilation.")
            .required(false))
        .arg(Arg::with_name("input")
            .short("i")
            .long("input")
            .required(true)
            .value_name("FILE")
            .help("Input file for reading code."))
        .get_matches();
    let jit_enabled = matches.is_present("jit");
    let path = matches.value_of("input").expect("No input file specified. See --help");
    let mut file = std::fs::File::open(path).unwrap();
    let mut code = String::new();
    file.read_to_string(&mut code).unwrap();

    let lex = Token::lexer(&code);

    let mut parser: Parser = Parser::new( lex);
    while let Ok(_) = parser.parse_expr() {/*Call the function until it can no longer be called.*/}

    let context = Context::create();
    let module = context.create_module("MeidoLang");
    let builder = context.create_builder();
    let execution_engine = module.create_jit_execution_engine(OptimizationLevel::None).unwrap();
    let mut codegen = Compiler {
        context: &context,
        builder: builder.borrow(),
        module: module.borrow(),
        variables: vec![],
        execution_engine: execution_engine.borrow(),
        printf_defined: false,
        string_count: 1,
        print_stack_count: 1
    };

    codegen.build_main();
    parser.stack.reverse();
    while let Some(an_expr) = parser.stack.pop() {
        codegen.compile_expr(&an_expr).expect("Unable to compile a statement.");
    }
    codegen.build_end_return();

    if jit_enabled {
        let maybe_fn = unsafe { execution_engine.get_function::<unsafe extern "C" fn() -> i32>("main") };
        let compiled_fn = match maybe_fn {
            Ok(f) => f,
            Err(err) => {
                println!("!> Error during execution: {:?}", err);
                return ()
            }
        };
        unsafe {
            println!("=> {}", compiled_fn.call());
        }
    }
    else {
        codegen.write_to_file().expect("Could not write to file.");
    }

}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    #[test]
    fn lexer_lexes_a_program() {
        let mut lex = Token::lexer("レムレムラムベティレムレムラムベティ+ベティスバルtest君さよなら.");

        assert_eq!(lex.next(), Some(Token::ONE));
            assert_eq!(lex.span(), 0..6);
            assert_eq!(lex.slice(), "レム");

            assert_eq!(lex.next(), Some(Token::ONE));
            assert_eq!(lex.span(), 6..12);
            assert_eq!(lex.slice(), "レム");

            assert_eq!(lex.next(), Some(Token::FINALIZER));
            assert_eq!(lex.span(), 12..18);
            assert_eq!(lex.slice(), "ラム");
    }
}
