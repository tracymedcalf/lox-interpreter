use crate::ast::Ast;
use crate::interpreter::Interpreter;
use crate::parser::Parser;
use crate::resolver::Resolver;
use crate::scanner::Scanner;
use crate::token::{Token, TokenKind};

pub fn new_var(s: &str) -> Token {
    Token {
        kind: TokenKind::Identifier,
        line: 0,
        content: s.to_string(),
    }
}

pub fn scan_parse(s: &str) -> Ast {
    let tokens = Scanner::new(s.to_string()).scan_tokens();
    match Parser::new(tokens.clone()).parse() {
        Ok(ast) => ast,
        Err(err) => panic!("Parse failed: {:?}\n{:?}", err, tokens),
    }
}

pub fn test_run(code: &str) -> Interpreter {
    let mut ast = scan_parse(code);
    println!("{:#?}", ast);
    let mut resolver = Resolver::new();
    resolver.run(&mut ast).unwrap();
    let mut interpreter = Interpreter::new();
    interpreter.run(ast).unwrap();
    interpreter
}
