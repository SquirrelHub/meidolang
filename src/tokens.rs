
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
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