use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interp_error::{InterpError, InterpResult};
use crate::token::Token;
use crate::value::*;

type Link = Rc<RefCell<Node>>;
type Scope = HashMap<String, Value>;

#[derive(PartialEq)]
struct Node {
    parent: Option<Link>,
    scope: Scope,
}

impl Node {
    fn new_link() -> Link {
        Node::new_with_scope(HashMap::new())
    }

    fn new_with_parent(parent: Link) -> Link {
        Rc::new(RefCell::new(Node {
            parent: Some(parent),
            scope: HashMap::new(),
        }))
    }

    fn new_with_scope(scope: HashMap<String, Value>) -> Link {
        Rc::new(RefCell::new(Node {
            parent: None,
            scope,
        }))
    }
}

pub struct Environment {
    current: Link,
}

impl Clone for Environment {
    fn clone(&self) -> Environment {
        Environment {
            current: self.current.clone()
        }
    }
}


impl Environment {
    pub fn new() -> Environment {
        Environment {
            current: Node::new_link()
        }
    }

    pub fn assign(&mut self, token: &Token, value: Value) -> InterpResult {
        let mut bn = self.current.borrow_mut();
        if bn.scope.contains_key(&token.content) {
            bn.scope.insert(token.content.clone(), value.clone());
            Ok(value)
        } else {
            Err(InterpError::new("Variable not found in scope.", token.clone()))
        }
    }

    pub fn assign_at(&mut self, depth: u32, name: String, value: Value) {
        self.ancestor(depth)
            .borrow_mut()
            .scope
            .insert(name, value)
            .unwrap();
    }

    pub fn bind_this(&mut self, object: &Object) {
        self.insert("this", Value::Object(object.clone()));
    }

    pub fn declare_and_assign(&mut self, token: &Token, new_value: Value) {
        self.current
            .borrow_mut()
            .scope
            .insert(token.content.clone(), new_value);
    }

    pub fn get(&self, token: &Token) -> InterpResult {
        if let Some(value) = self.current.borrow().scope.get(&token.content) {
            Ok(value.clone())
        } else {
            Err(InterpError::new("Variable not found.", token.clone()))
        }
    } 

    pub fn get_at(&self, depth: u32, name: &str) -> Value {
        println!("getting ... {}", name);
        self.ancestor(depth)
            .borrow()
            .scope
            .get(name)
            .unwrap()
            .clone()
    }

    pub fn insert(&mut self, key: &str, value: Value) {
        self.current
            .borrow_mut()
            .scope
            .insert(key.to_string(), value);
    }

    pub fn maybe_get_at(&self, depth: u32, name: &str) -> Option<Value> {
        if let Some(value) = self.ancestor(depth)
            .borrow()
            .scope
            .get(name) {
                Some(value.clone())
        } else {
            None
        }
    }

    pub fn bind_arguments(&mut self, arguments: Vec<Value>, parameters: &Vec<Token>) {
        for (arg, param) in arguments.into_iter().zip(parameters) {
            self.declare_and_assign(param, arg);
        }
    }
    
    pub fn new_block(&self) -> Environment {
        Environment {
            current: Node::new_with_parent(self.current.clone())
        }
    }

    pub fn new_with_values(values: HashMap<String, Value>) -> Environment {
        Environment {
            current: Node::new_with_scope(values) 
        }
    }
    
    fn ancestor(&self, depth: u32) -> Link {
        let mut node = self.current.clone();
        for _ in 0..depth {
            node = {
                let borrowed_node = node.borrow();
                borrowed_node.parent.as_ref().unwrap().clone()
            }
        }
        node
    }
}

