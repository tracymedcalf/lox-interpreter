use std::collections::{HashMap, VecDeque};

use crate::ast::*;
use crate::error::report;
use crate::token::*;
use TokenKind::*;

pub struct Parser {
    tokens: VecDeque<Token>,
    previous: Option<Token>,
}

type ExprResult = Result<Expr, ParseErr>;
type StatementResult = Result<Statement, ParseErr>;
type DeclarationResult = Result<Declaration, ParseErr>;
type AstResult = Result<Ast, ()>;

struct ParseErr {
    line: usize,
    message: String,
}

impl ParseErr {
    fn new(token: &Token, message: &str) -> ParseErr {
        ParseErr {
            line: token.line,
            message: message.to_string(),
        }
    }

    fn report(&self) {
        report(self.line, &self.message);
    }
}

impl Parser {
    fn error(&mut self, message: &str) -> ParseErr {
        self.advance();
        ParseErr::new(&self.previous(), &format!("Parse error: {}", message))
    }

    fn synchronize(&mut self) {
        self.advance();
        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }

            match self.peek().kind {
                TokenKind::Class
                | TokenKind::Fun
                | TokenKind::Var
                | TokenKind::For
                | TokenKind::If
                | TokenKind::While
                | TokenKind::Print
                | TokenKind::Return => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    fn consume(&mut self, token_kind: TokenKind, message: &str) -> Result<(), ParseErr> {
        if self.check(token_kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn is_at_end(&self) -> bool {
        self.tokens.len() == 0
    }

    fn peek(&self) -> &Token {
        &self.tokens[0]
    }

    fn previous(&mut self) -> Token {
        self.previous.clone().unwrap()
    }

    fn advance(&mut self) {
        if let Some(token) = self.tokens.pop_front() {
            self.previous = Some(token);
        }
    }

    fn equal(&mut self, types: Vec<TokenKind>) -> bool {
        for t in types {
            if self.check(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, t: TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().kind == t
        }
    }

    fn expression(&mut self) -> ExprResult {
        self.assignment()
    }

    fn primary(&mut self) -> ExprResult {
        if self.equal(vec![False, True, Nil, Number, StringT]) {
            Ok(Expr::new_literal(self.previous()))
        } else if self.equal(vec![Identifier]) {
            Ok(Expr::new_variable(self.previous()))
        } else if self.equal(vec![LeftParen]) {
            let expr = self.expression()?;
            // TODO: Switch to new way of handling errors.
            self.consume(TokenKind::RightParen, "Expected ')' after expression.")?;
            Ok(Expr::new_grouping(self.previous(), expr))
        } else if self.equal(vec![This]) {
            Ok(Expr::new_this(self.previous()))
        } else if self.equal(vec![Super]) {
            let token = self.previous();
            self.consume(Dot, "Expect '.' after 'super'.")?;
            self.consume(Identifier, "Expected identifier after '.'")?;
            let method = self.previous();
            Ok(Expr::new_super(method, token))
        } else {
            Err(self.error("Expected expression."))
        }
    }

    fn call(&mut self) -> ExprResult {
        let mut expr = self.primary()?;
        loop {
            if self.equal(vec![LeftParen]) {
                expr = self.finish_call(expr)?;
            } else if self.equal(vec![Dot]) {
                self.consume(Identifier, "Expected property name after '.'.")?;
                let token = self.previous();
                expr = if self.equal(vec![Equal]) {
                    let value = self.expression()?;
                    Expr::new_set(token, expr, value)
                } else {
                    Expr::new_get(token, expr)
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> ExprResult {
        let mut arguments = Vec::new();
        if !self.check(RightParen) {
            loop {
                if arguments.len() >= 255 {
                    return Err(self.error("Can't have more than 255 arguments"));
                }
                arguments.push(self.expression()?);
                if self.equal(vec![Comma]) {
                    break;
                }
            }
        }
        self.consume(RightParen, "Expected closing paren to follow argument list")?;
        Ok(Expr::new_call(callee, arguments, self.previous()))
    }

    fn unary(&mut self) -> ExprResult {
        if self.equal(vec![Bang, Minus]) {
            let operator = self.previous();
            let right = self.unary()?;
            Ok(Expr::new_unary(operator, right))
        } else {
            self.call()
        }
    }

    fn factor(&mut self) -> ExprResult {
        let mut expr = self.unary()?;
        while self.equal(vec![Slash, Star]) {
            let operator = self.previous();
            let right = self.factor()?;
            let expr2 = Expr::new_binary(expr, operator, right);
            expr = expr2;
        }
        Ok(expr)
    }

    fn term(&mut self) -> ExprResult {
        let mut expr = self.factor()?;
        while self.equal(vec![Minus, Plus]) {
            let operator = self.previous();
            let right = self.factor()?;
            let expr2 = Expr::new_binary(expr, operator, right);
            expr = expr2;
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> ExprResult {
        let mut expr = self.term()?;
        while self.equal(vec![Greater, GreaterEqual, Less, LessEqual]) {
            let operator = self.previous();
            let right = self.comparison()?;
            let expr2 = Expr::new_binary(expr, operator, right);
            expr = expr2;
        }
        Ok(expr)
    }

    fn equality(&mut self) -> ExprResult {
        let mut expr = self.comparison()?;
        while self.equal(vec![BangEqual, EqualEqual]) {
            let operator = self.previous();
            let right = self.comparison()?;
            expr = Expr::new_binary(expr, operator, right);
        }
        Ok(expr)
    }

    fn consume_semicolon(&mut self) -> Result<(), ParseErr> {
        self.consume(Semicolon, "Semicolon must follow statement.")
    }

    fn if_statement(&mut self) -> StatementResult {
        let cond = self.equality()?;
        let true_branch = self.statement()?;
        let else_branch = if self.equal(vec![Else]) {
            let block = self.statement()?;
            Some(block)
        } else {
            None
        };
        Ok(Statement::new_if(cond, true_branch, else_branch))
    }

    fn while_statement(&mut self) -> StatementResult {
        self.consume(LeftParen, "Expected '(' following 'while'")?;
        let cond = self.equality()?;
        self.consume(RightParen, "Expected ')' following condition")?;
        let body = self.statement()?;
        Ok(Statement::new_while(cond, body))
    }

    fn block(&mut self) -> Result<Vec<Declaration>, ParseErr> {
        let mut declarations: Vec<Declaration> = Vec::new();
        // TODO: Currently, confusing error messages occur when right brace
        // is left off of block.
        while !self.equal(vec![RightBrace]) {
            let new_element = self.declaration()?;
            declarations.push(new_element);
        }
        Ok(declarations)
    }

    fn print_statement(&mut self) -> StatementResult {
        let expr = self.expression()?;
        self.consume_semicolon()?;
        Ok(Statement::new_print(expr))
    }

    fn expr_statement(&mut self) -> ExprResult {
        let value = self.expression()?;
        self.consume_semicolon()?;
        Ok(value)
    }

    fn for_statement(&mut self) -> StatementResult {
        self.consume(LeftParen, "Expected '(' following 'for'")?;
        let initializer = if self.equal(vec![Semicolon]) {
            None
        } else if self.equal(vec![Var]) {
            Some(Initializer::VarDeclaration(self.var_declaration()?))
        } else {
            Some(Initializer::Expr(self.expr_statement()?))
        };

        let cond = if !self.check(Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume_semicolon()?;

        let increment = Some(self.expression()?);
        self.consume(RightParen, "Expected ')' following condition")?;
        let body = self.statement()?;
        Ok(Statement::new_for(initializer, cond, increment, body))
    }

    fn return_statement(&mut self) -> StatementResult {
        let value = if !self.check(Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(Semicolon, "Expected semicolon after 'return'")?;
        Ok(Statement::Return(value))
    }

    fn statement(&mut self) -> StatementResult {
        if self.equal(vec![Print]) {
            self.print_statement()
        } else if self.equal(vec![LeftBrace]) {
            Ok(Statement::new_block(self.block()?))
        } else if self.equal(vec![If]) {
            self.if_statement()
        } else if self.equal(vec![While]) {
            self.while_statement()
        } else if self.equal(vec![For]) {
            self.for_statement()
        } else if self.equal(vec![Return]) {
            self.return_statement()
        } else {
            Ok(Statement::new_expr_statement(self.expr_statement()?))
        }
    }

    fn and(&mut self) -> ExprResult {
        let mut expr = self.equality()?;
        while self.equal(vec![And]) {
            let operator = self.previous();
            let right = self.and()?;
            expr = Expr::new_logical(expr, operator, right);
        }
        Ok(expr)
    }

    fn or(&mut self) -> ExprResult {
        let mut expr = self.and()?;
        while self.equal(vec![Or]) {
            let operator = self.previous();
            let right = self.and()?;
            expr = Expr::new_logical(expr, operator, right);
        }
        Ok(expr)
    }

    fn assignment(&mut self) -> ExprResult {
        let expr = self.or()?;
        if self.equal(vec![TokenKind::Equal]) {
            let rvalue = self.assignment()?;
            if let ExprKind::Variable(_) = expr.kind {
                // TODO: In the future, this will have to be reworked to take something other than a
                // token (think assigning to object fields)
                return Ok(Expr::new_assign(expr.token, rvalue));
            }

            return Err(self.error("Invalid assignment target."));
        }
        Ok(expr)
    }

    fn var_declaration(&mut self) -> Result<VarDeclaration, ParseErr> {
        self.consume(Identifier, "Expected variable name.")?;
        let name = self.previous();

        let initializer = if self.equal(vec![Equal]) {
            let expr = self.expression()?;
            Some(expr)
        } else {
            None
        };

        self.consume_semicolon()?;
        Ok(VarDeclaration::new(name, initializer))
    }

    fn function(&mut self, s: &str) -> Result<FunDeclaration, ParseErr> {
        self.consume(Identifier, &format!("Expected {} name.", s))?;
        let name = self.previous();
        self.consume(LeftParen, &format!("Expect '(' after {} name.", s))?;
        let mut parameters = Vec::new();
        if !self.check(RightParen) {
            loop {
                self.consume(Identifier, "Expected parameter name.")?;
                if parameters.len() >= 255 {
                    return Err(self.error("Can't have more than 255 parameters"));
                }
                parameters.push(self.previous());
                if !self.equal(vec![Comma]) {
                    break;
                }
            }
        }
        self.consume(RightParen, "Expected ')' to follow '('")?;
        self.consume(LeftBrace, &format!("Expected '{{' before {} body", s))?;
        let body = self.block()?;
        Ok(FunDeclarationStruct::new_fun_declaration(name, parameters, body))
    }
    
    fn class(&mut self) -> DeclarationResult {
        self.consume(Identifier, "Expected class name")?;
        let name = self.previous();
        let superclass = if self.equal(vec![Less]) {
            self.consume(Identifier, "Expected class name.")?;
            Some(Expr::new_variable(self.previous()))
        } else {
            None
        };
        self.consume(LeftBrace, "Expected left brace")?;
        let mut methods = HashMap::new();
        while !self.is_at_end() && !self.check(RightBrace) {
            let function = self.function("method")?;
            let name = {
                function.borrow().name.content.clone()
            };
            methods.insert(name, function);
        }
        self.consume(RightBrace, "Expected right brace.")?;
        Ok(Declaration::new_class(methods, name, superclass))
    }

    fn declaration(&mut self) -> DeclarationResult {
        if self.equal(vec![Class]) {
            self.class()
        } else if self.equal(vec![Var]) {
            Ok(Declaration::VarDeclaration(self.var_declaration()?))
        } else if self.equal(vec![Fun]) {
            let function = self.function("function")?;
            Ok(Declaration::FunDeclaration(function))
        } else {
            let statement = self.statement()?;
            Ok(Declaration::Statement(statement))
        }
    }

    pub fn parse(&mut self) -> AstResult {
        let mut declarations: Vec<Declaration> = Vec::new();
        let mut had_error = false;
        while !self.is_at_end() {
            match self.declaration() {
                Ok(declaration) => declarations.push(declaration),
                Err(parse_error) => {
                    parse_error.report();
                    had_error = true;
                    self.synchronize();
                }
            }
        }
        if had_error {
            Err(())
        } else {
            Ok(Ast { declarations })
        }
    }

    pub fn new(tokens: VecDeque<Token>) -> Parser {
        Parser {
            tokens,
            previous: None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils::*;
    #[test]
    fn test_if() {
        let _ast = scan_parse(
            "
            if (1 == 2)
                print(\"What?\");
            else
                print(\"The universe continues to make sense\");",
        );
    }

    #[test]
    fn test_parse() {
        let _ast = scan_parse(
            "
        if (1) {
            var a = 1;
            print(a);
        }",
        );
    }

    #[test]
    fn test_fun() {
        let s = "
        my_f();";
        scan_parse(s);
    }

    #[test]
    fn test_fun_declaration() {
        let s = "
        fun f(a, b, c) {
            print(\"hey there!\");
        }";
        scan_parse(s);
    }
}
