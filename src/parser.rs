
use logos::Logos;
use crate::tokens::Token;

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

    ProgramEnd,

    StringPrint(Box<String>)
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Val {
    pub n: i32
}

pub struct Parser<'a> {
    lex: logos::Lexer<'a, Token>,
    current: Option<Token>,
    pub stack: Vec<Box<Expr>>,
    variables: Vec<Box<Expr>>
}

impl<'a> Parser<'a> {
    pub fn new(mut l: logos::Lexer<'a, Token>) -> Self {
        let cur = l.next();
        Parser {
            lex: l,
            current: cur,
            stack: Vec::new(),
            variables: Vec::new()
        }
    }

    pub fn parse_expr(&mut self) -> Result<Expr, &'static str> {
        if self.current == Some(Token::PLUS) ||
            self.current == Some(Token::MINUS) ||
            self.current == Some(Token::MULT) ||
            self.current == Some(Token::DIV) {
            let val = self.parse_binary_expr();
            let return_val = val.clone();
            let value = Box::from(Box::from(val.clone().expect("Value was not present.")));
            self.stack.push(*value);
            self.variables.push(Box::new(val.unwrap()).clone());
            return return_val;
        } else if self.current == Some(Token::ONE) {
            let val = self.parse_nb_expr().unwrap();
            let value = Box::new(Expr::Number(Box::new(val)));
            self.variables.push(value.clone());
            self.stack.push(value);
            return Ok(Expr::Number(Box::new(val)));
        } else if self.current == Some(Token::STRINGSTART) {
            self.parse_string_expr()
        } else if self.current == Some(Token::PRINTSTACK) {
            self.parse_print_stack_expr()
        } else if self.current == Some(Token::PROGRAMEND) {
            //Ignore everything else. Program should terminate.
            self.lex = Logos::lexer("");
            self.current = self.lex.next();
            self.stack.push(Box::new(Expr::ProgramEnd));
            Ok(Expr::ProgramEnd)
        } else {
            Err("Unknown At this time.")
        }
    }

    fn parse_binary_expr(&mut self) -> Result<Expr, &'static str> {
        if self.variables.len() < 2 {
            return Err("Not enough variables to perform an operation")
        }
        let right = self.stack.pop().expect("Strange Value Exists in the stack.");
        let left = self.stack.pop().expect("Strange Value Exists in the stack.");
        self.variables.pop();
        self.variables.pop();
        let op: char;
        if self.current == Some(Token::PLUS) {
            op = '+';
        } else if self.current == Some(Token::MINUS) {
            op = '-';
        } else if self.current == Some(Token::MULT) {
            op = '*';
        } else if self.current == Some(Token::DIV) {
            op = '/';
        } else {
            return Err("Unknown Operator");
        }
        self.current = self.lex.next();

        Ok(Expr::Binary {
            op,
            left,
            right
        })
    }

    fn parse_nb_expr(&mut self) -> Result<Val, &'static str> {
        if self.current == Some(Token::ONE) {
            self.current = self.lex.next();
            return Ok(Val {
                n: 1 + self.parse_nb_expr().unwrap().n
            });
        } else if self.current == Some(Token::FINALIZER) {
            self.current = self.lex.next();
            return Ok(Val { n: 0 });
        } else {
            return Err("Number was broken.")
        }
    }

    fn parse_string_expr(&mut self) -> Result<Expr, &'static str> {
        self.current = self.lex.next();
        if self.current != Some(Token::STRINGLITERAL) {
            Err("No String found.")
        } else {
            let the_string = self.lex.slice().to_string() + &*" ";
            self.current = self.lex.next();
            if self.current != Some(Token::STRINGEND) {
                Err("String was not ended properly.")
            } else {
                self.current = self.lex.next();
                if self.stack.len() > 0 {
                    let call = Expr::Call {
                        other: self.stack.pop().unwrap(),
                        actual: Box::new(Expr::StringPrint(Box::new(the_string)))
                    };
                    self.stack.push(Box::from(call.clone()));
                    Ok(call)
                } else {
                    self.stack.push(Box::new(Expr::StringPrint(Box::new(the_string.clone()))));
                    Ok(Expr::StringPrint(Box::new(the_string)))
                }
            }
        }
    }

    fn parse_print_stack_expr(&mut self) -> Result<Expr, &'static str> {
        self.current = self.lex.next();
        if self.stack.len() > 0 {
            let call = Expr::Call {
                other: self.stack.pop().unwrap(),
                actual: Box::new(Expr::PrintStack)
            };
            self.stack.push(Box::from(call.clone()));
            Ok(call)
        } else {
            self.stack.push(Box::new(Expr::PrintStack));
            Ok(Expr::PrintStack)
        }
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
        assert_eq!(parse.parse_expr().unwrap(), Expr::Binary {
            op: '+',
            left: Box::new(Expr::Number(Box::new(Val { n: 2 }))),
            right: Box::new(Expr::Number(Box::new(Val { n: 2 })))
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
        assert_eq!(parse.parse_binary_expr().unwrap(), Expr::Binary {
            op: '*',
            left: Box::new(Expr::Number(Box::new(Val { n: 2 }))),
            right: Box::new(Expr::Number(Box::new(Val { n: 2 })))
        })
    }

    #[test]
    fn parse_binary_expr_parses_div_operation() {
        let lex = Token::lexer("レムレムラムレムレムラム/");
        let mut parse: Parser = Parser::new(lex);
        parse.parse_expr();
        parse.parse_expr();
        assert_eq!(parse.parse_binary_expr().unwrap(), Expr::Binary {
            op: '/',
            left: Box::new(Expr::Number(Box::new(Val { n: 2 }))),
            right: Box::new(Expr::Number(Box::new(Val { n: 2 })))
        })
    }

    #[test]
    fn parse_binary_expr_parses_multiple_operations() {
        let lex = Token::lexer("レムレムラムレムレムラム+レムラム-");
        let mut parse: Parser = Parser::new(lex);
        parse.parse_expr();
        parse.parse_expr();
        assert_eq!(parse.parse_expr().unwrap(), Expr::Binary {
            op: '+',
            left: Box::new(Expr::Number(Box::new(Val { n: 2 }))),
            right: Box::new(Expr::Number(Box::new(Val { n: 2 })))
        });
        parse.parse_expr();
        assert_eq!(parse.parse_expr().unwrap(), Expr::Binary {
            op: '-',
            left: Box::new(Expr::Binary {
                op: '+',
                left: Box::new(Expr::Number(Box::new(Val { n: 2 }))),
                right: Box::new(Expr::Number(Box::new(Val { n: 2 })))
            }),
            right: Box::new(Expr::Number(Box::new(Val { n: 1 })))
        });
    }
}