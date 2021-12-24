###MeidoLang (メイドラング)

A not so useful and esoteric language.

The goal of this project was to contain some quirky or novel syntax in a stack-style programming language.
The behavior of this language borrows heavily from a language I briefly looked into called [gforth](https://www.gnu.org/software/gforth/).

The language is not turing-complete, and therefore is not likely useful for any real-world task. Operations supported include:

* "+"
* "-"
* "*"
* "/"
* Printing the stack
* Printing the string

The grammar as I understand it (Hopefully I understand it. It is my language after all.) looks like this:

```
program: expList PROGRAMEND
    ;

expList: exp expList 
    | exp
    ;
    
exp:
    value FINALIZER
    | exp exp PLUS
    | exp exp MINUS
    | exp exp MULT
    | exp exp DIV
    | PRINTSTACK
    |  STRINGSTART STRINGLITERAL STRINGEND
    ;

value: value ONE 
    | ONE
    ;
```

The language utilizes llvm through the `inkwell` wrapper of the rust bindings. If you have been struggling
to learn how to use it, hopefully this code is useful to you!

Note: If you are trying to use this language, although I have not tested it, I would recommend you have a `C` compiler
lying around, presuming then you've also got those libraries around as well. The print code makes use of `printf`, so
I would imagine having `C` installed on your system would be important.