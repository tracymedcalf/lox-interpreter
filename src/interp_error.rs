use crate::token::Token;
use crate::value::Value;

pub type InterpResult = Result<Value, InterpError>;

#[derive(PartialEq, Debug)]
pub struct Error {
    message: String,
    token: Token,
}

impl Error {
    pub fn new(message: &str, token: Token) -> Error {
        Error {
            message: message.to_string(),
            token,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum InterpError {
    Error(Error),
    Return(Value),
}

impl InterpError {
    pub fn new(message: &str, token: Token) -> InterpError {
        InterpError::Error(Error::new(message, token))
    }
}

pub type StatementResult = Result<(), InterpError>;
