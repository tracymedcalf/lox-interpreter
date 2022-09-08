use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::cmp;
use std::rc::Rc;

use crate::ast::FunDeclaration;
use crate::environment::Environment;

pub type IClass = Rc<RefCell<IClassStruct>>;

#[derive(Debug, PartialEq)]
pub struct IClassStruct {
    pub name: String,
    pub methods: HashMap<String, UserDefined>,
    pub superclass: Option<IClass>,
}

impl IClassStruct {
    pub fn new_i_class(methods: HashMap<String, UserDefined>, name: &str, superclass: Option<IClass>) -> IClass {
        Rc::new(RefCell::new(IClassStruct {
            methods,
            name: name.to_string(),
            superclass,
        }))
    }
}

pub type Object = Rc<RefCell<ObjectStruct>>;

#[derive(Debug, PartialEq)]
pub struct ObjectStruct {
    pub class: IClass,
    pub fields: HashMap<String, Value>,
}

impl ObjectStruct {
    pub fn new_object(class: &IClass) -> Object {
        Rc::new(RefCell::new(ObjectStruct {
            class: class.clone(),
            fields: HashMap::new(),
        }))
    }
}

#[derive(Clone)]
pub struct UserDefined {
    pub declaration: FunDeclaration,
    pub environment: Environment,
    pub is_initializer: bool,
}

impl fmt::Debug for UserDefined {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UserDefined")
    }
}

impl cmp::PartialEq for UserDefined {
   fn eq(&self, other: &UserDefined) -> bool {
       self.declaration == other.declaration
   }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    UserDefined(UserDefined),
    Builtin,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Boolean(bool),
    Class(IClass),
    Function(Function),
    Nil,
    Number(f64),
    Object(Object),
    StringV(String),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Nil => false,
            _ => true,
        }
    }

    pub fn new_function(declaration: &FunDeclaration, environment: Environment, is_initializer: bool) -> Value {
        Value::Function(Function::UserDefined(Value::new_user_defined(declaration, environment, is_initializer)))
    }

    pub fn new_user_defined(declaration: &FunDeclaration, environment: Environment, is_initializer: bool) -> UserDefined {
        UserDefined {
            environment,
            declaration: declaration.clone(),
            is_initializer,
        }
    }

    pub fn to_string(self) -> String {
        match self {
            Value::Boolean(b) => format!("{}", b),
            Value::Class(class) => format!("CLASS {:?}", class.borrow()),
            Value::Function(_function) => "FUNCTION".to_string(),
            Value::Nil => "nil".to_string(),
            Value::Number(n) => format!("{}", n),
            Value::Object(object) => format!("Instance of {:?}", object.borrow().class.borrow().name),
            Value::StringV(s) => s,
        }
    }
}
