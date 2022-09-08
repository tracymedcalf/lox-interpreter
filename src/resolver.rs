use std::collections::{HashMap, VecDeque};

use crate::ast::*;
use crate::interp_error::Error;
use crate::token::Token;
use Status::*;

type ResolverResult = Result<(), Error>;

fn error(message: &str, token: Token) -> ResolverResult {
    Err(Error::new(message, token))
}

enum Status {
    Declared,
    Defined,
}

pub struct Resolver {
    scopes: VecDeque<HashMap<String, Status>>,
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {
            scopes: VecDeque::new(),
        }
    }

    pub fn run(&mut self, ast: &mut Ast) -> ResolverResult {
        self.visit_declarations(&mut ast.declarations)
    }

    fn begin_scope(&mut self) {
        self.scopes.push_front(HashMap::new());
    }

    fn declare(&mut self, token: &Token) {
        if let Some(scope) = self.scopes.front_mut() {
            scope.insert(token.content.clone(), Declared);
        }
    }

    fn define(&mut self, token: &Token) {
        if let Some(scope) = self.scopes.front_mut() {
            scope.insert(token.content.clone(), Defined);
        }
    }

    fn end_scope(&mut self) {
        let _ = self.scopes.pop_front();
    }

    fn visit_assign_expr(&mut self, assign_expr: &mut AssignExpr, token: &Token) -> ResolverResult {
        self.visit_expr(&mut assign_expr.initializer)?;
        self.resolve_local(&mut assign_expr.depth, token)?;
        Ok(())
    }

    fn visit_binary_expr(&mut self, binary_expr: &mut BinaryExpr) -> ResolverResult {
        self.visit_expr(&mut binary_expr.left)?;
        self.visit_expr(&mut binary_expr.right)?;
        Ok(())
    }

    fn visit_block(&mut self, block: &mut Vec<Declaration>) -> ResolverResult {
        self.begin_scope();
        self.visit_declarations(block)?;
        self.end_scope();
        Ok(())
    }

    fn visit_call(&mut self, call: &mut Call) -> ResolverResult {
        self.visit_expr(&mut call.callee)?;
        for expr in call.arguments.iter_mut() {
            self.visit_expr(expr)?;
        }
        Ok(())
    }

    fn visit_class(&mut self, class: &mut Class) -> ResolverResult {
        let mut class_struct = class.borrow_mut();
        if let ClassStruct { name, superclass: Some(superclass_expr), methods: _ } = &mut *class_struct {
            if superclass_expr.token.content == name.content {
                return error("A class cannot inherit from itself.", superclass_expr.token.clone());
            } else {
                if let Expr { kind: ExprKind::Variable(depth), token } = superclass_expr {
                    self.resolve_local(depth, token)?;
                } else {
                    panic!();
                }
            }
        }
        self.define(&(*class_struct).name);
        if class_struct.superclass.is_some() {
            let super_scope = hashmap!["super".to_string() => Status::Defined];
            self.scopes.push_front(super_scope);
        }
        let scope = hashmap!["this".to_string() => Status::Defined];
        self.scopes.push_front(scope);
        for f in class_struct.methods.values_mut() {
            self.visit_fun_declaration(f)?;
        }
        self.end_scope();
        if class_struct.superclass.is_some() {
            self.end_scope();
        }
        Ok(())
    }

    fn visit_declarations(&mut self, declarations: &mut Vec<Declaration>) -> ResolverResult {
        for declaration in declarations {
            self.visit_declaration(declaration)?;
        }
        Ok(())
    }

    fn visit_declaration(&mut self, declaration: &mut Declaration) -> ResolverResult {
        match declaration {
            Declaration::Class(class) => self.visit_class(class),
            Declaration::FunDeclaration(fun_declaration) => {
                self.visit_fun_declaration(fun_declaration)
            }
            Declaration::Statement(statement) => self.visit_statement(statement),
            Declaration::VarDeclaration(var_declaration) => {
                self.visit_var_declaration(var_declaration)
            }
        }
    }

    fn visit_expr(&mut self, expr: &mut Expr) -> ResolverResult {
        match expr {
            Expr {
                kind: ExprKind::Assign(ref mut assign_expr),
                token,
            } => self.visit_assign_expr(assign_expr, token),
            Expr {
                kind: ExprKind::Binary(ref mut binary_expr),
                token: _,
            } => self.visit_binary_expr(binary_expr),
            Expr {
                kind: ExprKind::Call(call),
                token: _,
            } => self.visit_call(call),
            Expr {
                kind: ExprKind::Get(object),
                token: _,
            } => self.visit_expr(object),
            Expr {
                kind: ExprKind::Grouping(expr),
                token: _,
            } => self.visit_expr(expr),
            Expr {
                kind: ExprKind::Literal,
                token: _,
            } => Ok(()),
            Expr {
                kind: ExprKind::Logical(ref mut binary_expr),
                token: _,
            } => self.visit_binary_expr(binary_expr),
            Expr {
                kind: ExprKind::Set(ref mut set),
                token: _,
            } => self.visit_set(set),
            Expr {
                kind: ExprKind::This(depth),
                token,
            } => self.visit_this(depth, token),
            Expr {
                kind: ExprKind::Unary(ref mut inner_expr),
                token: _,
            } => self.visit_expr(inner_expr),
            Expr {
                kind: ExprKind::Variable(depth),
                token,
            } => self.visit_var_expr(depth, token),
            Expr {
                kind: ExprKind::Super(_method, depth),
                token,
            } => self.visit_super(depth, token),
        }
    }

    fn visit_for_statement(&mut self, for_statement: &mut For) -> ResolverResult {
        self.begin_scope();
        if let Some(initializer) = &mut for_statement.initializer {
            self.visit_initializer(initializer)?;
        }
        self.visit_option_expr(&mut for_statement.cond)?;
        self.visit_option_expr(&mut for_statement.increment)?;
        self.visit_statement(&mut for_statement.body)?;
        self.end_scope();
        Ok(())
    }

    fn visit_fun_declaration(&mut self, fun_declaration: &mut FunDeclaration) -> ResolverResult {
        let mut fun_declaration = fun_declaration.borrow_mut();
        self.declare(&fun_declaration.name);
        self.begin_scope();
        for param in &fun_declaration.params {
            self.define(param);
        }
        self.visit_declarations(&mut fun_declaration.body)?;
        self.end_scope();
        self.define(&fun_declaration.name);
        Ok(())
    }

    fn visit_if_statement(&mut self, if_statement: &mut If) -> ResolverResult {
        self.visit_expr(&mut if_statement.cond)?;
        self.visit_statement(&mut if_statement.true_branch)?;
        if let Some(else_branch) = &mut if_statement.else_branch {
            self.visit_statement(else_branch)
        } else {
            Ok(())
        }
    }

    fn visit_initializer(&mut self, initializer: &mut Initializer) -> ResolverResult {
        match initializer {
            Initializer::VarDeclaration(var_declaration) => {
                self.visit_var_declaration(var_declaration)
            }
            Initializer::Expr(expr) => self.visit_expr(expr),
        }
    }

    fn visit_option_expr(&mut self, option_expr: &mut Option<Expr>) -> ResolverResult {
        if let Some(expr) = option_expr {
            self.visit_expr(expr)
        } else {
            Ok(())
        }
    }

    fn visit_set(&mut self, set: &mut Set) -> ResolverResult {
        self.visit_expr(&mut set.object)?;
        self.visit_expr(&mut set.value)
    }

    fn visit_this(&mut self, depth: &mut Option<u32>, token: &Token) -> ResolverResult {
        self.resolve_local(depth, token)
    }

    fn visit_var_declaration(&mut self, declaration: &mut VarDeclaration) -> ResolverResult {
        self.declare(&declaration.name);
        if let Some(initializer) = &mut declaration.initializer {
            self.visit_expr(initializer)?;
        }
        self.define(&declaration.name);
        Ok(())
    }

    fn visit_var_expr(&mut self, depth: &mut Option<u32>, token: &Token) -> ResolverResult {
        if let Some(scope) = self.scopes.front() {
            if let Some(Declared) = scope.get(&token.content) {
                return error(
                    "Can't read local variable in its own initializer",
                    token.clone(),
                );
            }
        }
        self.resolve_local(depth, token)?;
        Ok(())
    }

    fn visit_return_expr(&mut self, return_expr: &mut Option<Expr>) -> ResolverResult {
        if let Some(expr) = return_expr {
            self.visit_expr(expr)
        } else {
            Ok(())
        }
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> ResolverResult {
        match statement {
            Statement::Block(declarations) => self.visit_block(declarations),
            Statement::ExprStatement(expr) => self.visit_expr(expr),
            Statement::If(if_statement) => self.visit_if_statement(&mut **if_statement),
            Statement::For(for_statement) => self.visit_for_statement(for_statement),
            Statement::Print(expr) => self.visit_expr(expr),
            Statement::Return(return_expr) => self.visit_return_expr(return_expr),
            Statement::While(while_statement) => self.visit_while_statement(while_statement),
        }
    }

    fn visit_super(&mut self, depth: &mut Option<u32>, token: &Token) -> ResolverResult {
        self.resolve_local(depth, token)
    }

    fn visit_while_statement(&mut self, while_statement: &mut While) -> ResolverResult {
        self.visit_expr(&mut while_statement.cond)?;
        self.visit_statement(&mut while_statement.body)
    }

    fn resolve_local(&mut self, depth: &mut Option<u32>, token: &Token) -> ResolverResult {
        for (i, scope) in self.scopes.iter().enumerate() {
            if scope.contains_key(&token.content) {
                if let Ok(new_depth) = u32::try_from(i) {
                    let _ = std::mem::replace(depth, Some(new_depth));
                } else {
                    return error("Exceeded maximum scope depth.", token.clone());
                }
            }
        }
        Ok(())
    }
}
