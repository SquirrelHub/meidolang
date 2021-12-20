#![allow(unused_variables)]

use logos::Logos;

extern crate inkwell;

use std::borrow::Borrow;
use std::io::{self, Write};
use std::thread::current;
use inkwell::AddressSpace;
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
use inkwell::values::FunctionValue;


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

    Number(Box<Val>),

    PrintStack,

    StringPrint
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
    variables: Vec<IntValue<'ctx>>,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {

    pub fn init_targets() {
        Target::initialize_all(&InitializationConfig::default())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<IntValue<'ctx>, &'static str> {
        match &expr {
            Expr::Number(nb) => {
                Ok(self.context.i32_type().const_int(nb.n as u64, true))
            },
            Expr::Binary {op, ref left, ref right} => {
                let lhs = self.compile_expr(left)?;
                let rhs = self.compile_expr(right)?;
                match op {
                    '+' => {
                        Ok(self.builder.build_int_add(lhs, rhs, "anAdd"))
                    },
                    '-' => Ok(self.builder.build_int_sub(lhs, rhs, "anSub")),
                    '*' => Ok(self.builder.build_int_mul(lhs, rhs, "anMult")),
                    '/' => Ok(self.builder.build_int_signed_div(lhs, rhs, "anDiv")),
                    _ => Err("Invalid operator. Check parser did not parse incorrectly.")
                }
            }
            _ => Err("Code Generation Failed. Can't generate at this time, or invalid program was being built.")
        }
    }

    fn build_main(&self) {
        let main_fn_type = self.context.i32_type().fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_fn_type, Some(Linkage::External));
        let basic_block = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(basic_block);

        let i32_type = self.context.i32_type();
        let i32_zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&i32_zero));
    }

    fn write_to_file(&self) -> Result<(), String> {
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
                RelocMode::Default,
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
    //let mut lex = Token::lexer("レムレムラムベティレムレムラムベティ+ベティスバルtest君さよなら.");
    let lex = Token::lexer("レムレムラムレムレムラム+レムラム-");
    let mut parser: Parser = Parser::new(lex);
    let mut ast = Expr::StringPrint;
    while let Ok(i) = parser.parse_expr() {
        ast = i;
    }


    let context = Context::create();
    let module = context.create_module("MeidoLang");
    let mut builder = context.create_builder();
    let execution_engine = module.create_jit_execution_engine(OptimizationLevel::None);
    let mut codegen = Compiler {
        context: &context,
        builder: builder.borrow(),
        module: module.borrow(),
        variables: vec![]
    };

    codegen.build_main();
    //let some_val = codegen.compile_expr(&ast);
    codegen.write_to_file();
    ()
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
