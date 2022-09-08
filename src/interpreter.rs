use std::collections::HashMap;
use std::time::SystemTime;

use crate::ast::*;
use crate::environment::*;
use crate::interp_error::{InterpError, InterpResult, StatementResult};
use crate::token::{Token, TokenKind};
use crate::value::*;

macro_rules! number_operation {
    ($value1: expr, $value2: expr, $operator: tt, $token: expr) => {
        if let Value::Number(n1) = $value1 {
            if let Value::Number(n2) = $value2 {
                return Ok(Value::Number(n1 $operator n2));
            }
        }
        return Err(InterpError::new("Expected number in expression.", $token.clone()));
    }

}

macro_rules! number_comparison {
    ($value1: expr, $value2: expr, $operator: tt, $token: expr) => {
        if let Value::Number(n1) = $value1 {
            if let Value::Number(n2) = $value2 {
                return Ok(Value::Boolean(n1 $operator n2))
            }
        }
        return Err(InterpError::new("Expected number in expression.", $token.clone()));
    }
}

type DeclarationResult = Result<(), InterpError>;

impl Token {
    fn visit(&self) -> InterpResult {
        let v = match &self.kind {
            TokenKind::Number => {
                let n = self.content.parse::<f64>().unwrap();
                Value::Number(n)
            }
            TokenKind::StringT => Value::StringV(self.content.clone()),
            TokenKind::True => Value::Boolean(true),
            TokenKind::False => Value::Boolean(false),
            TokenKind::Nil => Value::Nil,
            _ => unreachable!(),
        };
        Ok(v)
    }
}

pub struct Interpreter {
    globals: Environment,
    start: SystemTime,
}

impl Interpreter {
    pub fn new() -> Interpreter {
        let values = hashmap!{
            "clock".to_string() => Value::Function(Function::Builtin),
        };
        Interpreter {
            globals: Environment::new_with_values(values),
            start: SystemTime::now(),
        }
    }

    pub fn run(&mut self, ast: Ast) -> StatementResult {
        let mut environment = self.globals.clone();
        for declaration in &ast.declarations {
            self.visit_declaration(declaration, &mut environment)?;
        }
        Ok(())
    }

    fn assign_global(&mut self, token: &Token, value: Value) -> InterpResult {
        self.globals.assign(token, value)
    }

    fn declare_and_assign(&mut self, environment: &mut Environment, token: &Token, new_value: Value) {
        environment.declare_and_assign(token, new_value);
    }

    fn finish_class(&mut self, class: ClassStruct) {}

    fn visit_class(&mut self, class: &Class, environment: &mut Environment) -> DeclarationResult {
        let borrowed_class = class.borrow();
        let (methods, superclass) = if let Some(Expr { token, kind: ExprKind::Variable(depth) }) = &borrowed_class.superclass {
            println!("Storing superclass");
            let superclass_value = self.visit_var_expr(depth, environment, token)?;
            let mut environment = environment.new_block();
            environment.insert("super", superclass_value.clone());
            (generate_methods(&borrowed_class.methods, &mut environment),
            if let Value::Class(i_superclass) = superclass_value {
                Some(i_superclass.clone())
            } else {
                return Err(InterpError::new("Can only inherit from classes", token.clone()));
            })
        } else {
            (generate_methods(&borrowed_class.methods, environment),
            None)
        };
        let class_struct = IClassStruct::new_i_class(methods, &borrowed_class.name.content, superclass);
        self.declare_and_assign(environment, &class.borrow().name, Value::Class(class_struct));
        Ok(())
    }

    fn visit_declaration(&mut self, declaration: &Declaration, environment: &mut Environment) -> DeclarationResult {
        match declaration {
            Declaration::Class(class) => self.visit_class(class, environment),
            Declaration::FunDeclaration(fun_declaration) => {
                self.visit_fun_declaration(environment, fun_declaration)
            }
            Declaration::Statement(statement) => self.visit_statement(environment, statement),
            Declaration::VarDeclaration(var_declaration) => {
                self.visit_var_declaration(environment, var_declaration)
            }
        }
    }

    fn visit_declarations(&mut self, declarations: &Vec<Declaration>, environment: &mut Environment) -> DeclarationResult {
        for d in declarations {
            self.visit_declaration(d, environment)?;
        }
        Ok(())
    }

    fn visit_var_declaration(&mut self, environment: &mut Environment, var_declaration: &VarDeclaration) -> DeclarationResult {
        let value = if let Some(expr) = &var_declaration.initializer {
            self.visit_expr(environment, expr)?
        } else {
            Value::Nil
        };
        self.declare_and_assign(environment, &var_declaration.name, value);
        Ok(())
    }

    fn visit_for(&mut self, environment: &mut Environment, for_statement: &For) -> StatementResult {
        if let Some(initializer) = &for_statement.initializer {
            self.visit_initializer(environment, initializer)?;
        }

        if let Some(cond) = &for_statement.cond {
            self.visit_expr(environment, cond)?;
        }

        let mut bool_value = Value::Boolean(true);
        if let Some(cond) = &for_statement.cond {
            bool_value = self.visit_expr(environment, cond)?;
        }

        while bool_value.is_truthy() {
            self.visit_statement(environment, &for_statement.body)?;

            if let Some(increment) = &for_statement.increment {
                self.visit_expr(environment, increment)?;
            }

            if let Some(cond) = &for_statement.cond {
                bool_value = self.visit_expr(environment, cond)?;
            }
        }

        Ok(())
    }

    fn visit_fun_declaration(&mut self, environment: &mut Environment, fun_declaration: &FunDeclaration) -> DeclarationResult {
        let new_function = Value::new_function(fun_declaration, environment.clone(), false);
        let fun_declaration = fun_declaration.borrow();
        self.declare_and_assign(environment, &fun_declaration.name, new_function);
        Ok(())
    }

    fn visit_block(&mut self, declarations: &Vec<Declaration>, environment: &mut Environment) -> StatementResult {
        let result = self.visit_declarations(declarations, &mut environment.new_block());
        result
    }

    fn visit_statement(&mut self, environment: &mut Environment, statement: &Statement) -> StatementResult {
        match statement {
            Statement::ExprStatement(expr) => {
                self.visit_expr(environment, expr)?;
                Ok(())
            }
            Statement::Print(expr) => {
                let value = self.visit_expr(environment, expr)?;
                println!("{}", value.to_string());
                Ok(())
            }
            Statement::Block(declarations) => self.visit_block(declarations, environment),
            Statement::If(if_statement) => {
                let bool_value = self.visit_expr(environment, &if_statement.cond)?;
                if bool_value.is_truthy() {
                    self.visit_statement(environment, &if_statement.true_branch)?;
                } else {
                    if let Some(else_branch) = &if_statement.else_branch {
                        self.visit_statement(environment, else_branch)?;
                    }
                }

                Ok(())
            }
            Statement::While(while_statement) => {
                let mut bool_value = self.visit_expr(environment, &while_statement.cond)?;
                while bool_value.is_truthy() {
                    self.visit_statement(environment, &while_statement.body)?;
                    bool_value = self.visit_expr(environment, &while_statement.cond)?;
                }

                Ok(())
            }
            Statement::For(for_statement) => {
                self.visit_for(&mut environment.new_block(), &for_statement)
            }
            Statement::Return(return_value) => {
                let value = match return_value {
                    Some(expr) => self.visit_expr(environment, expr)?,
                    None => Value::Nil,
                };
                Err(InterpError::Return(value))
            }
        }
    }

    fn visit_initializer(&mut self, environment: &mut Environment, initializer: &Initializer) -> InterpResult {
        match initializer {
            Initializer::VarDeclaration(var_declaration) => {
                self.visit_var_declaration(environment, var_declaration)?;
                Ok(Value::Nil)
            }
            Initializer::Expr(expr) => self.visit_expr(environment, expr),
        }
    }

    fn visit_binary_expr(&mut self, binary_expr: &BinaryExpr, environment: &mut Environment, token: &Token) -> InterpResult {
        let left_v = self.visit_expr(environment, &binary_expr.left)?;
        let right_v = self.visit_expr(environment, &binary_expr.right)?;

        match &token.kind {
            TokenKind::Plus => match left_v {
                Value::StringV(left_s) => {
                    if let Value::StringV(right_s) = right_v {
                        Ok(Value::StringV(format!("{}{}", left_s, right_s)))
                    } else {
                        Err(InterpError::new(
                                "Expected string in concatenation operation.",
                                token.clone(),
                        ))
                    }
                }
                Value::Number(left_n) => {
                    if let Value::Number(right_n) = right_v {
                        Ok(Value::Number(left_n + right_n))
                    } else {
                        Err(InterpError::new(
                                "Expected number in expression.",
                                token.clone(),
                        ))
                    }
                }
                _ => Err(InterpError::new("Invalid operation.", token.clone())),
            },
            TokenKind::Minus => {
                number_operation!(left_v, right_v, -, token);
            }
            TokenKind::Star => {
                number_operation!(left_v, right_v, *, token);
            }
            TokenKind::Slash => {
                number_operation!(left_v, right_v, /, token);
            }
            TokenKind::BangEqual => Ok(Value::Boolean(left_v != right_v)),
            TokenKind::EqualEqual => Ok(Value::Boolean(left_v == right_v)),
            TokenKind::LessEqual => {
                number_comparison!(left_v, right_v, <=, token);
            }
            TokenKind::Less => {
                number_comparison!(left_v, right_v, <, token);
            }
            TokenKind::GreaterEqual => {
                number_comparison!(left_v, right_v, >=, token);
            }
            TokenKind::Greater => {
                number_comparison!(left_v, right_v, >, token);
            }
            _ => unreachable!(),
        }
    }

    fn visit_unary(&mut self, environment: &mut Environment, expr: &Expr, token: &Token) -> InterpResult {
        let value = self.visit_expr(environment, expr)?;
        match &token.kind {
            TokenKind::Minus => {
                if let Value::Number(n) = value {
                    Ok(Value::Number(-n))
                } else {
                    Err(InterpError::new(
                            "Expected number in expression.",
                            token.clone(),
                    ))
                }
            }
            TokenKind::Bang => Ok(Value::Boolean(!value.is_truthy())),
            _ => unreachable!(),
        }
    }

    fn visit_logical(&mut self, environment: &mut Environment, logical: &BinaryExpr, token: &Token) -> InterpResult {
        let left_v = self.visit_expr(environment, &logical.left)?;
        let boolean = match token.kind {
            TokenKind::And => left_v.is_truthy() && self.visit_expr(environment, &logical.right)?.is_truthy(),
            TokenKind::Or => left_v.is_truthy() || self.visit_expr(environment, &logical.right)?.is_truthy(),
            _ => unreachable!(),
        };

        Ok(Value::Boolean(boolean))
    }

    fn finish_call(
        &mut self,
        call: &Call,
        closing_paren: &Token,
        calling_environment: &mut Environment,
        function: Function,
    ) -> InterpResult {
        let mut arguments = Vec::new();
        for arg in &call.arguments {
            // TODO: 2 environments?
            arguments.push(self.visit_expr(calling_environment, arg)?);
        }
        match function {
            Function::UserDefined(rc) => {
                let declaration = rc.declaration.borrow();
                if arguments.len() != declaration.params.len() {
                    let msg = format!(
                        "Arity mismatch: declaration {} expected {} arguments, received {}.",
                        call.callee.token.content,
                        declaration.params.len(),
                        arguments.len()
                    );
                    return Err(InterpError::new(&msg, closing_paren.clone()));
                }
                let mut environment = rc.environment.new_block();
                println!("{:?}", environment.maybe_get_at(1, "this"));
                environment.bind_arguments(arguments, &declaration.params);
                let result = self.visit_declarations(&declaration.body, &mut environment);
                match result {
                    Ok(()) => {
                        if rc.is_initializer {
                            let this = rc.environment.get_at(0, "this");
                            Ok(this)
                        } else {
                            Ok(Value::Nil)
                        }
                    },
                    Err(InterpError::Return(value)) => Ok(value),
                    Err(error) => Err(error),
                }
            }
            Function::Builtin => match call.callee.token.content.as_str() {
                "clock" => {
                    let time = self.start.elapsed().unwrap();
                    Ok(Value::Number(time.as_millis() as f64))
                }
                _ => {
                    unreachable!();
                }
            },
        }
    }

    fn call_class(&mut self, call: &Call, class: &IClass, closing_paren: &Token) -> InterpResult {
        Ok(Value::Object(ObjectStruct::new_object(class)))
    }

    fn get_global(&mut self, token: &Token) -> InterpResult {
        self.globals.get(token)
    }

    fn visit_call(&mut self, call: &Call, closing_paren: &Token, environment: &mut Environment) -> InterpResult {
        let value = self.visit_expr(environment, &call.callee)?;
        println!("Call : {:?}", closing_paren);
        match value {
            Value::Function(function) => {
                self.finish_call(call, closing_paren, environment, function)
            },
            Value::Class(class) => {
                if let Some(user_defined) = class.borrow().methods.get("init") {
                    let object = ObjectStruct::new_object(&class);
                    let mut user_defined_clone = user_defined.clone();
                    user_defined_clone.environment.bind_this(&object);
                    let function = Function::UserDefined(user_defined_clone);
                    self.finish_call(call, &closing_paren, environment, function)
                } else {
                    self.call_class(call, &class, closing_paren)
                }
            },
            _ => {
                Err(InterpError::new(
                        "Can only call functions and classes.",
                        closing_paren.clone(),
                ))
            }
        }
    }

    fn visit_expr(&mut self, environment: &mut Environment, expr: &Expr) -> InterpResult {
        match expr {
            Expr {
                kind: ExprKind::Assign(assign_expr),
                token,
            } => self.visit_assign_expr(assign_expr, environment, token),
            Expr {
                kind: ExprKind::Binary(binary_expr),
                token,
            } => self.visit_binary_expr(binary_expr, environment, token),
            Expr {
                kind: ExprKind::Call(expr),
                token,
            } => self.visit_call(&*expr, token, environment),
            Expr {
                kind: ExprKind::Literal,
                token,
            } => token.visit(),
            Expr {
                kind: ExprKind::Logical(logical),
                token,
            } => self.visit_logical(environment, logical, token),
            Expr {
                kind: ExprKind::Get(object),
                token,
            } => self.visit_get(environment, object, token),
            Expr {
                kind: ExprKind::Grouping(expr),
                token: _,
            } => self.visit_expr(environment, expr),
            Expr {
                kind: ExprKind::Set(set),
                token,
            } => self.visit_set(environment, token, set),
            Expr {
                kind: ExprKind::This(depth),
                token,
            } => self.visit_this(depth, environment, token),
            Expr {
                kind: ExprKind::Unary(expr),
                token,
            } => self.visit_unary(environment, &*expr, token),
            Expr {
                kind: ExprKind::Variable(depth),
                token,
            } => self.visit_var_expr(depth, environment, token),
            Expr {
                kind: ExprKind::Super(method, depth),
                token,
            } => self.visit_super(depth, environment, method, token),
        }
    }

    fn visit_get(&mut self, environment: &mut Environment, object: &Expr, identifier: &Token) -> InterpResult {
        let value = self.visit_expr(environment, object)?;
        if let Value::Object(object) = value {
            ObjectStruct::get(&object, identifier)
        } else {
            Err(InterpError::new("Field access should be preceded by object.", identifier.clone()))
        }
    }

    fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, environment: &mut Environment, token: &Token) -> InterpResult {
        let value = self.visit_expr(environment, &*assign_expr.initializer)?;
        if let Some(depth) = assign_expr.depth {
            environment
                .assign_at(depth, token.content.clone(), value.clone());
            Ok(value)
        } else {
            self.assign_global(token, value)
        }
    }

    fn visit_set(&mut self, environment: &mut Environment, name: &Token, set: &Set) -> InterpResult {
        let left_value = self.visit_expr(environment, &set.object)?;
        if let Value::Object(object) = left_value {
            let right_value = self.visit_expr(environment, &set.value)?;
            println!("insert {}", &name.content);
            object.borrow_mut().fields.insert(name.content.clone(), right_value.clone());
            Ok(right_value)
        } else {
            Err(InterpError::new("Can only set properties of objects.", name.clone()))
        }
    }

    fn visit_this(&mut self, depth: &Option<u32>, environment: &mut Environment, this: &Token) -> InterpResult {
        if let Some(depth) = depth {
            Ok(environment.get_at(*depth, &this.content))
        } else {
            Err(InterpError::new("Cannot access this in global context.", this.clone()))
        }
    }

    fn visit_var_expr(&mut self, depth: &Option<u32>, environment: &mut Environment, token: &Token) -> InterpResult {
        if let Some(depth) = depth {
            Ok(environment.get_at(*depth, &token.content))
        } else {
            self.get_global(token)
        }
    }

    fn visit_super(&mut self, depth: &Option<u32>, environment: &mut Environment, method: &Token, token: &Token) -> InterpResult {
        let depth = depth.unwrap();
        let superclass_value = environment.get_at(depth, &token.content);
        if let Value::Object(object) = environment.get_at(depth - 1, "this") {
            if let Value::Class(superclass) = superclass_value {
                if let Some(method) = superclass.borrow().find_method(&method.content) {
                    environment.bind_this(&object);
                    Ok(Value::Function(Function::UserDefined(method)))
                } else {
                    Err(InterpError::new("Method not found on 'super'.", token.clone()))
                }
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
    }
}

impl IClassStruct {
    pub fn find_method(&self, content: &str) -> Option<UserDefined> {
        if let Some(method) = self.methods.get(content).clone() {
            println!("Getting method {} off of {:?}", content, self);
            Some(method.clone())
        } else {
            if let Some(superclass) = &self.superclass {
                superclass.borrow().find_method(content)
            } else {
                None
            }
        }
    }
}

impl ObjectStruct {
    pub fn get(object: &Object, identifier: &Token) -> InterpResult {
        let object_struct = object.borrow();
        if let Some(value) = object_struct.fields.get(&identifier.content) {
            Ok(value.clone())
        } else {
            if let Some(user_defined) = object_struct.class.borrow().find_method(&identifier.content) {
                let mut closure = user_defined.environment.new_block();
                closure.bind_this(object);
                let new_user_defined = Value::new_user_defined(
                    &user_defined.declaration,
                    closure,
                    user_defined.is_initializer
                );
                Ok(Value::Function(Function::UserDefined(new_user_defined)))
            } else {
                Err(InterpError::new("Property not found on object.", identifier.clone()))
            }
        }
    }
}
    
fn generate_methods(class_methods: &HashMap<String, FunDeclaration>, environment: &mut Environment) -> HashMap<String, UserDefined> {
    let mut methods = HashMap::new();
    for (name, fun_declaration) in class_methods {
        let new_function = Value::new_user_defined(fun_declaration, environment.clone(), name == "init");
        methods.insert(name.clone(), new_function);
    }
    methods
}


#[cfg(test)]
pub mod test_utils {
    use crate::test_utils::*;
    use crate::value::Value;
    pub fn test_interpret(code: &str, variable_name: &str) -> Value {
        test_run(code)
            .get_global(&new_var(variable_name))
            .expect("variable not found.")
    }
}
