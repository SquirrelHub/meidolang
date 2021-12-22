#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::any::Any;
use logos::Logos;

extern crate inkwell;

use std::borrow::Borrow;
use std::fmt::format;
use std::io::{self, Write};
use std::thread::current;
use clap::{App, Arg};
use inkwell::AddressSpace;
use inkwell::AddressSpace::Global;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Linkage;
use crate::Token::{MINUS, MULT, PLUS, PROGRAMEND};

use self::inkwell::builder::Builder;
use self::inkwell::context::Context;
use self::inkwell::module::Module;
use self::inkwell::passes::PassManager;
use self::inkwell::types::BasicMetadataTypeEnum;
use self::inkwell::values::{BasicValue, BasicMetadataValueEnum, IntValue};
use self::inkwell::{OptimizationLevel, FloatPredicate};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::values::{AnyValue, CallableValue, FunctionValue, GlobalValue};


#[derive(Logos, Debug, PartialEq)]
enum Token {
    // Tokens can be literal strings, of any length.
    #[token("レム")]
    ONE,

    #[token("ラム")]
    FINALIZER,

    #[token("スバル")]
    STRINGSTART,

    #[regex("[a-zA-Z]+")]
    STRINGLITERAL,

    #[token("君")]
    STRINGEND,

    #[token("ベティ")]
    PRINTSTACK,

    #[token("+")]
    PLUS,

    #[token("-")]
    MINUS,

    #[token("*")]
    MULT,

    #[token("/")]
    DIV,

    #[token("さよなら")]
    PROGRAMEND,

    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f\v]+", logos::skip)]
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Binary {
        op: char,
        left: Box<Expr>,
        right: Box<Expr>
    },

    Call {
        other: Box<Expr>,
        actual: Box<Expr>
    },

    Number(Box<Val>),

    PrintStack,

    StringPrint(Box<String>)
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Val {
    n: i32
}

pub struct Parser<'a> {
    lex: logos::Lexer<'a, Token>,
    current: Option<Token>,
    stack: Vec<Box<Expr>>
}

impl<'a> Parser<'a> {

    fn new(mut l: logos::Lexer<'a, Token>) -> Self {
        let cur = l.next();
        Parser{
            lex: l,
            current: cur,
            stack: Vec::new()
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, &'static str> {
        if self.current == Some(PLUS) ||
            self.current == Some(MINUS) ||
            self.current == Some(MULT) ||
            self.current == Some(Token::DIV) {
            let val = self.parse_binary_expr();
            let return_val = val.clone();
            let value = Box::from(Box::from(val.expect("Value was not present.")));
            self.stack.push(*value);
            return return_val;
        }
        else if self.current == Some(Token::ONE) {
            let val = self.parse_nb_expr().unwrap();
            let value = Box::new(Expr::Number(Box::new(val)));
            self.stack.push(value);
            return Ok(Expr::Number(Box::new(val)));
        }
        else if self.current == Some(Token::STRINGSTART){
            self.current = self.lex.next();
            if self.current != Some(Token::STRINGLITERAL) {
                Err("No String found.")
            }
            else {
                let the_string = self.lex.slice().to_string();
                self.current = self.lex.next();
                if self.current != Some(Token::STRINGEND){
                    Err("String was not ended properly.")
                }
                else {
                    self.current = self.lex.next();
                    if self.stack.len() > 0 {
                        let call = Expr::Call {
                            other: self.stack.pop().unwrap(),
                            actual: Box::new(Expr::StringPrint(Box::new(the_string)))
                        };
                        self.stack.push(Box::from(call.clone()));
                        Ok(call)
                    }
                    else {
                        Ok(Expr::StringPrint(Box::new(the_string)))
                    }
                }
            }
        }
        else if self.current == Some(Token::PRINTSTACK) {
            self.current = self.lex.next();
            if self.stack.len() > 0 {
                let call = Expr::Call {
                    other: self.stack.pop().unwrap(),
                    actual: Box::new(Expr::PrintStack)
                };
                self.stack.push(Box::from(call.clone()));
                Ok(call)
            }
            else {
                Ok(Expr::PrintStack)
            }
        }
        else{
            Err("Unknown At this time.")
        }
    }

    fn parse_binary_expr(&mut self) -> Result<Expr, &'static str> {
        if (self.stack.len() < 2){
            return(Err("Not enough variables to perform an operation"))
        }
        let right = self.stack.pop().expect("Strange Value Exists in the stack.");
        let left = self.stack.pop().expect("Strange Value Exists in the stack.");
        let op: char;
        if self.current == Some(Token::PLUS) {
            op = '+';
        }
        else if self.current == Some(Token::MINUS) {
            op = '-';
        }
        else if self.current == Some(Token::MULT) {
            op = '*';
        }
        else if self.current == Some(Token::DIV) {
            op = '/';
        }
        else {
           return Err("Unknown Operator");
        }
        self.current = self.lex.next();

        Ok(Expr::Binary {
                op,
                left,
                right
            })

    }

    /// Parses a literal number.
    fn parse_nb_expr(&mut self) -> Result<Val, &'static str> {
        if self.current == Some(Token::ONE){
            self.current = self.lex.next();
            return Ok(Val{
                n: 1 + self.parse_nb_expr().unwrap().n
            });

        }
        else if self.current == Some(Token::FINALIZER){
            self.current = self.lex.next();
            return Ok(Val{n:0});
        }
        else{
            return Err("Number was broken.")
        }
    }
}

pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
    pub variables: Vec<IntValue<'ctx>>,
    pub execution_engine: &'a ExecutionEngine<'ctx>,
    pub printf_defined: bool,
    pub string_count: u16,
    pub print_stack_count: u16
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {

    fn compile_expr(&mut self, expr: &Expr) -> Result<IntValue<'ctx>, &'static str> {
        match &expr {
            Expr::Number(nb) => {
                let return_val = self.context.i32_type().const_int(nb.n as u64, true);
                self.variables.push(return_val.clone());
                Ok(return_val)
            },
            Expr::Binary {op, ref left, ref right} => {
                let lhs = self.compile_expr(left)?;
                let rhs = self.compile_expr(right)?;
                self.variables.pop();
                self.variables.pop();
                match op {
                    '+' => {
                        let return_val = self.builder.build_int_add(lhs, rhs, "anAdd");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    '-' => {
                        let return_val = self.builder.build_int_sub(lhs, rhs, "aSub");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    '*' => {
                        let return_val = self.builder.build_int_mul(lhs, rhs, "aMult");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    '/' => {
                        let return_val = self.builder.build_int_signed_div(lhs, rhs, "aDiv");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    _ => Err("Invalid operator. Check parser did not parse incorrectly.")
                }
            }
            Expr::Call {ref other, ref actual} => {
                let some_expr = self.compile_expr(other);
                let the_expr = self.compile_expr(actual);
                Ok(some_expr.unwrap())
            }
            Expr::StringPrint(str) => {
                if !self.printf_defined {
                    self.define_printf();
                    self.printf_defined = true
                }
                let name_of_string = "string".to_string() + &self.string_count.to_string();
                let the_string = self.builder.build_global_string_ptr(str.as_str(), name_of_string.as_str());
                self.string_count += 1;
                let mut arguments: Vec<BasicMetadataValueEnum> = vec![];
                arguments.push(the_string.as_pointer_value().into());
                self.builder.build_call(self.module.get_function("printf").unwrap(), &arguments, "printf");
                Ok(self.context.i32_type().const_int(0, false))
            }
            Expr::PrintStack => {
                if !self.printf_defined {
                    self.define_printf();
                    self.printf_defined = true
                }
                let mut arguments : Vec<BasicMetadataValueEnum> = vec![];
                let mut format_string : String = "".to_string();
                let name_of_string = "print_stack".to_string() + &self.print_stack_count.to_string();
                for var in self.variables.clone() {
                    format_string = format_string + "%d ";
                }
                let the_string = self.builder.build_global_string_ptr(format_string.as_str(), name_of_string.as_str());
                self.print_stack_count += 1;
                arguments.push(the_string.as_pointer_value().into());
                for var in self.variables.clone() {
                    arguments.push(var.into());
                }

                self.builder.build_call(self.module.get_function("printf").unwrap(), &arguments, "printf");
                Ok(self.context.i32_type().const_int(0, false))
            }
            _ => Err("Code Generation Failed. Can't generate at this time, or invalid program was being built.")
        }
    }

    fn define_printf(&self) {
        let printf_fn_type = self.context.i32_type().fn_type(&[self.context.ptr_sized_int_type(self.execution_engine.get_target_data(), Option::from(Global)).into()], true);
        self.module.add_function("printf", printf_fn_type, Some(Linkage::External)).set_call_conventions(0); // https://llvm.org/doxygen/namespacellvm_1_1CallingConv.html
    }

    fn build_main(&self) {
        let main_fn_type = self.context.i32_type().fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_fn_type, Some(Linkage::External));
        let basic_block = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(basic_block);
    }

    fn build_end_return(&self) {
        let i32_type = self.context.i32_type();
        let i32_zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&i32_zero));
    }

    fn write_to_file(&self) -> Result<(), String> {
        Target::initialize_all(&InitializationConfig::default());
        let target_triple = TargetMachine::get_default_triple();
        let cpu = TargetMachine::get_host_cpu_name().to_string();
        let features = TargetMachine::get_host_cpu_features().to_string();

        let target = Target::from_triple(&target_triple).map_err(|e| format!("{:?}", e))?;
        let target_machine = target
            .create_target_machine(
                &target_triple,
                &cpu,
                &features,
                OptimizationLevel::Default,
                RelocMode::DynamicNoPic,
                CodeModel::Default,
            )
            .ok_or_else(|| "Unable to create target machine!".to_string())?;

        let buff = target_machine
            .write_to_memory_buffer(&self.module, FileType::Assembly)
            .expect("couldn't compile to assembly");

        println!(
            "Assembly:\n{}",
            String::from_utf8(buff.as_slice().to_vec()).unwrap()
        );

        target_machine
            .write_to_file(&self.module, FileType::Object, "a.o".as_ref())
            .map_err(|e| format!("{:?}", e))
    }
}

fn main() {

    let matches = App::new("MeidoLang")
        .version("0.1.0")
        .arg(Arg::with_name("jit")
            .short("j")
            .long("jit")
            .help("Specifies to run with just in time compilation.")
            .required(false))
        .get_matches();
    let jit_enabled = matches.is_present("jit");
    //let mut lex = Token::lexer("レムレムラムベティレムレムラムベティ+ベティスバルtest君さよなら.");

    let lex = Token::lexer("スバルtest君レムレムラムレムレムラム+レムラム-スバルtest君ベティ");

    let mut parser: Parser = Parser::new(lex);
    let mut ast = Expr::PrintStack;
    while let Ok(i) = parser.parse_expr() {
        ast = i;
    }


    let context = Context::create();
    let module = context.create_module("MeidoLang");
    let mut builder = context.create_builder();
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
    let body = codegen.compile_expr(&ast);
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
        codegen.write_to_file();
    }

}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    #[test]
    fn parse_nb_expr_parses_a_number() {
        let lex = Token::lexer("レムラム");
        let mut parse: Parser = Parser::new(lex);
        assert_eq!(parse.parse_nb_expr().unwrap().n, 1)
    }

    #[test]
    fn parse_nb_expr_parses_a_larger_number() {
        let lex = Token::lexer("レムレムレムレムラム");
        let mut parse: Parser = Parser::new(lex);
        assert_eq!(parse.parse_nb_expr().unwrap().n, 4)
    }

    #[test]
    fn parse_binary_expr_parses_plus_operation() {
        let lex = Token::lexer("レムレムラムレムレムラム+");
        let mut parse: Parser = Parser::new(lex);
        parse.parse_expr();
        parse.parse_expr();
        assert_eq!(parse.parse_expr().unwrap(), Expr::Binary{
            op: '+',
            left: Box::new(Expr::Number(Box::new(Val{n:2}))),
            right: Box::new(Expr::Number(Box::new(Val{n:2})))
        })
    }

    #[test]
    fn parse_binary_expr_parses_minus_operation() {
        let lex = Token::lexer("レムレムラムレムレムラム-");
        let mut parse: Parser = Parser::new(lex);
        parse.parse_expr();
        parse.parse_expr();
        assert_eq!(parse.parse_binary_expr().unwrap(), Expr::Binary {
            op: '-',
            left: Box::new(Expr::Number(Box::new(Val { n: 2 }))),
            right: Box::new(Expr::Number(Box::new(Val { n: 2 })))
        })
    }

        #[test]
    fn parse_binary_expr_parses_mult_operation() {
        let lex = Token::lexer("レムレムラムレムレムラム*");
        let mut parse: Parser = Parser::new(lex);
        parse.parse_expr();
        parse.parse_expr();
        assert_eq!(parse.parse_binary_expr().unwrap(), Expr::Binary{
            op: '*',
            left: Box::new(Expr::Number(Box::new(Val{n:2}))),
            right: Box::new(Expr::Number(Box::new(Val{n:2})))
        })
        }

    #[test]
    fn parse_binary_expr_parses_div_operation() {
        let lex = Token::lexer("レムレムラムレムレムラム/");
        let mut parse: Parser = Parser::new(lex);
        parse.parse_expr();
        parse.parse_expr();
        assert_eq!(parse.parse_binary_expr().unwrap(), Expr::Binary{
            op: '/',
            left: Box::new(Expr::Number(Box::new(Val{n:2}))),
            right: Box::new(Expr::Number(Box::new(Val{n:2})))
        })
    }

    #[test]
    fn parse_binary_expr_parses_multiple_operations() {
        let lex = Token::lexer("レムレムラムレムレムラム+レムラム-");
        let mut parse: Parser = Parser::new(lex);
        parse.parse_expr();
        parse.parse_expr();
        assert_eq!(parse.parse_expr().unwrap(), Expr::Binary{
        op: '+',
        left: Box::new(Expr::Number(Box::new(Val{n:2}))),
        right: Box::new(Expr::Number(Box::new(Val{n:2})))
        });
        parse.parse_expr();
        assert_eq!(parse.parse_expr().unwrap(), Expr::Binary{
            op: '-',
            left: Box::new(Expr::Binary{
                op: '+',
                left: Box::new(Expr::Number(Box::new(Val{n:2}))),
                right: Box::new(Expr::Number(Box::new(Val{n:2})))
            }),
            right: Box::new(Expr::Number(Box::new(Val{n:1})))
        });
    }

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
