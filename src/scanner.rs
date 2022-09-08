use std::collections::{HashMap, VecDeque};

use crate::error::report;
use crate::token::{Token, TokenKind};

fn is_digit(c: char) -> bool {
    '0' <= c && c <= '9'
}

fn is_alpha(c: char) -> bool {
    ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') || c == '_'
}

fn is_alpha_numeric(c: char) -> bool {
    is_digit(c) || is_alpha(c)
}

pub struct Scanner {
    source: String,
    start: usize,
    line: usize,
    current: usize,
    keywords: HashMap<String, TokenKind>,
}

impl Scanner {
    pub fn new(source: String) -> Scanner {
        let keywords = hashmap! {
            "and".to_string() => TokenKind::And,
            "class".to_string() => TokenKind::Class,
            "else".to_string() => TokenKind::Else,
            "false".to_string() => TokenKind::False,
            "for".to_string() => TokenKind::For,
            "fun".to_string() => TokenKind::Fun,
            "if".to_string() => TokenKind::If,
            "nil".to_string() => TokenKind::Nil,
            "or".to_string() => TokenKind::Or,
            "print".to_string() => TokenKind::Print,
            "return".to_string() => TokenKind::Return,
            "super".to_string() => TokenKind::Super,
            "this".to_string() => TokenKind::This,
            "true".to_string() => TokenKind::True,
            "var".to_string() => TokenKind::Var,
            "while".to_string() => TokenKind::While,
        };

        Scanner {
            source,
            current: 0,
            line: 0,
            start: 0,
            keywords,
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn advance(&mut self) -> char {
        let r = self.source.chars().nth(self.current).unwrap();
        self.current += 1;
        r
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source.chars().nth(self.current).unwrap()
        }
    }

    fn equal(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        if self.source.chars().nth(self.current).unwrap() != expected {
            return false;
        }

        self.current += 1;
        true
    }

    fn string(&mut self) -> String {
        let mut s = String::new();
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            s.push(self.advance());
        }
        if self.is_at_end() {
            report(self.line, "Unterminated string.");
        } else {
            self.advance();
        }
        s
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source.chars().nth(self.current + 1).unwrap()
        }
    }

    fn number(&mut self) -> String {
        let mut s = String::new();
        while is_digit(self.peek()) {
            s.push(self.advance());
        }

        if self.peek() == '.' && is_digit(self.peek_next()) {
            s.push(self.advance());
        }

        while is_digit(self.peek()) {
            s.push(self.advance());
        }
        s
    }

    fn identifier(&mut self) -> String {
        let mut s = String::new();
        while is_alpha_numeric(self.peek()) {
            s.push(self.advance());
        }
        s
    }

    fn scan_token(&mut self) -> Token {
        let c = self.advance();
        let mut content = "".to_string();
        let kind: TokenKind = match c {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '-' => TokenKind::Minus,
            '+' => TokenKind::Plus,
            ';' => TokenKind::Semicolon,
            '*' => TokenKind::Star,
            '/' if self.equal('/') => {
                while self.peek() != '\n' && !self.is_at_end() {
                    self.advance();
                }
                TokenKind::Comment
            }
            '/' => TokenKind::Slash,
            '!' if self.equal('=') => TokenKind::BangEqual,
            '!' => TokenKind::Bang,
            '=' if self.equal('=') => TokenKind::EqualEqual,
            '=' => TokenKind::Equal,
            '<' if self.equal('=') => TokenKind::LessEqual,
            '<' => TokenKind::Less,
            '>' if self.equal('=') => TokenKind::GreaterEqual,
            '>' => TokenKind::Greater,
            ' ' | '\r' | '\t' => TokenKind::WhiteSpace,
            '\n' => {
                self.line += 1;
                TokenKind::WhiteSpace
            }
            '"' => {
                content = self.string();
                TokenKind::StringT
            }
            c if is_digit(c) => {
                content = self.number();
                content.insert(0, c);
                TokenKind::Number
            }
            c if is_alpha(c) => {
                content.push(c);
                content = content + &self.identifier();
                if let Some(keyword) = self.keywords.get(&content) {
                    keyword.clone()
                } else {
                    TokenKind::Identifier
                }
            }
            _ => {
                let mut msg = "Unexpected character: ".to_string();
                msg.push(c);
                report(self.line, &msg);
                TokenKind::Error
            }
        };
        Token {
            line: self.line,
            kind,
            content,
        }
    }

    pub fn scan_tokens(&mut self) -> VecDeque<Token> {
        let mut tokens = VecDeque::new();
        while !self.is_at_end() {
            self.start = self.current;
            let t = self.scan_token();
            if let TokenKind::WhiteSpace = t.kind {
            } else {
                tokens.push_back(t);
            }
        }
        tokens
    }
}

#[test]
fn test_alpha_numeric() {
    assert!(is_alpha('a'));
    assert!(is_alpha('t'));
    assert!(is_digit('1'));
}
