use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::token::Token;

#[derive(Debug)]
pub struct Ast {
    pub declarations: Vec<Declaration>,
}

pub type Class = Rc<RefCell<ClassStruct>>;
pub type Depth = Option<u32>;

#[derive(Debug)]
pub struct ClassStruct {
    pub methods: HashMap<String, FunDeclaration>,
    pub name: Token,
    pub superclass: Option<Expr>,
}

impl PartialEq for ClassStruct {
    fn eq(&self, other: &ClassStruct) -> bool {
        self.name == other.name
    }
}

#[derive(Debug)]
pub enum Declaration {
    Class(Class),
    Statement(Statement),
    VarDeclaration(VarDeclaration),
    FunDeclaration(FunDeclaration),
}

impl Declaration {
    pub fn new_class(
        methods: HashMap<String, FunDeclaration>,
        name: Token,
        superclass: Option<Expr>,
    ) -> Declaration {
        Declaration::Class(Rc::new(RefCell::new(ClassStruct {
            methods,
            name,
            superclass,
        })))
    }

}

impl FunDeclarationStruct {
    pub fn new_fun_declaration(
        name: Token,
        params: Vec<Token>,
        body: Vec<Declaration>,
    ) -> FunDeclaration {
        Rc::new(RefCell::new(FunDeclarationStruct {
            body,
            name,
            params,
        }))
    }
}

#[derive(Debug)]
pub struct VarDeclaration {
    pub name: Token,
    pub initializer: Option<Expr>,
}

#[derive(Debug)]
pub struct FunDeclarationStruct {
    pub body: Vec<Declaration>,
    pub name: Token,
    pub params: Vec<Token>,
}

pub type FunDeclaration = Rc<RefCell<FunDeclarationStruct>>;

impl PartialEq for FunDeclarationStruct {
    fn eq(&self, other: &FunDeclarationStruct) -> bool {
        self.name == other.name
    }
}

impl VarDeclaration {
    pub fn new(name: Token, initializer: Option<Expr>) -> VarDeclaration {
        VarDeclaration { initializer, name }
    }
}

#[derive(Debug)]
pub struct If {
    pub cond: Expr,
    pub true_branch: Statement,
    pub else_branch: Option<Statement>,
}

#[derive(Debug)]
pub struct While {
    pub cond: Expr,
    pub body: Statement,
}

#[derive(Debug)]
pub enum Initializer {
    VarDeclaration(VarDeclaration),
    Expr(Expr),
}

#[derive(Debug)]
pub struct For {
    pub initializer: Option<Initializer>,
    pub cond: Option<Expr>,
    pub increment: Option<Expr>,
    pub body: Statement,
}

#[derive(Debug)]
pub enum Statement {
    Block(Vec<Declaration>),
    ExprStatement(Expr),
    For(Box<For>),
    If(Box<If>),
    Print(Expr),
    Return(Option<Expr>),
    While(Box<While>),
}

impl Statement {
    pub fn new_print(expr: Expr) -> Statement {
        Statement::Print(expr)
    }

    pub fn new_expr_statement(expr: Expr) -> Statement {
        Statement::ExprStatement(expr)
    }

    pub fn new_block(declarations: Vec<Declaration>) -> Statement {
        Statement::Block(declarations)
    }

    pub fn new_if(cond: Expr, true_branch: Statement, else_branch: Option<Statement>) -> Statement {
        Statement::If(Box::new(If {
            cond,
            true_branch,
            else_branch,
        }))
    }

    pub fn new_while(cond: Expr, body: Statement) -> Statement {
        Statement::While(Box::new(While { cond, body }))
    }

    pub fn new_for(
        initializer: Option<Initializer>,
        cond: Option<Expr>,
        increment: Option<Expr>,
        body: Statement,
    ) -> Statement {
        Statement::For(Box::new(For {
            initializer,
            cond,
            increment,
            body,
        }))
    }
}

#[derive(Debug)]
pub struct AssignExpr {
    pub depth: Depth,
    pub initializer: Box<Expr>,
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub token: Token,
}

#[derive(Debug)]
pub struct Call {
    pub arguments: Vec<Expr>,
    pub callee: Expr,
}

#[derive(Debug)]
pub struct Set {
    pub object: Expr,
    pub value: Expr,
}

#[derive(Debug)]
pub enum ExprKind {
    Assign(AssignExpr),
    Binary(Box<BinaryExpr>),
    Call(Box<Call>),
    Get(Box<Expr>),
    Grouping(Box<Expr>),
    Literal,
    Logical(Box<BinaryExpr>),
    Set(Box<Set>),
    This(Option<u32>),
    Unary(Box<Expr>),
    Variable(Option<u32>),
    Super(Token, Option<u32>),
}

impl Expr {
    fn new(kind: ExprKind, token: Token) -> Expr {
        Expr { kind, token }
    }

    pub fn new_assign(token: Token, expr: Expr) -> Expr {
        let kind = ExprKind::Assign(AssignExpr {
            depth: None,
            initializer: Box::new(expr),
        });
        Expr::new(kind, token)
    }

    pub fn new_binary(left: Expr, operator: Token, right: Expr) -> Expr {
        let kind = ExprKind::Binary(Box::new(BinaryExpr { left, right }));

        Expr::new(kind, operator)
    }

    pub fn new_call(callee: Expr, arguments: Vec<Expr>, closing_paren: Token) -> Expr {
        let kind = ExprKind::Call(Box::new(Call {
            arguments,
            callee,
        }));

        Expr::new(kind, closing_paren)
    }

    pub fn new_get(identifier: Token, object: Expr) -> Expr {
        let kind = ExprKind::Get(Box::new(object));
        Expr::new(kind, identifier)
    }

    pub fn new_grouping(beginning: Token, expr: Expr) -> Expr {
        let kind = ExprKind::Grouping(Box::new(expr));

        Expr::new(kind, beginning)
    }

    pub fn new_literal(token: Token) -> Expr {
        Expr::new(ExprKind::Literal, token)
    }

    pub fn new_logical(left: Expr, operator: Token, right: Expr) -> Expr {
        let kind = ExprKind::Logical(Box::new(BinaryExpr { left, right }));

        Expr::new(kind, operator)
    }

    pub fn new_set(name: Token, object: Expr, value: Expr) -> Expr {
        let kind = ExprKind::Set(Box::new(Set {
            object,
            value,
        }));
        Expr::new(kind, name)
    }

    pub fn new_this(token: Token) -> Expr {
        let kind = ExprKind::This(None);
        Expr::new(kind, token)
    }

    pub fn new_unary(operator: Token, expr: Expr) -> Expr {
        let kind = ExprKind::Unary(Box::new(expr));
        Expr::new(kind, operator)
    }

    pub fn new_variable(token: Token) -> Expr {
        let kind = ExprKind::Variable(None);
        Expr::new(kind, token)
    }

    pub fn new_super(method: Token, token: Token) -> Expr {
        let kind = ExprKind::Super(method, None);
        Expr::new(kind, token)
    }
}

#[derive(Debug)]
pub struct BinaryExpr {
    pub left: Expr,
    pub right: Expr,
}

impl Ast {
    pub fn print(&self) {
        println!("{:?}", self);
    }
}
