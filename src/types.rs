use std::fmt;

#[cfg(feature = "withserde")]
pub use serde_json::{Map, Number, Value};

//#[cfg(not(feature="serde"))]
use std::collections::HashMap;

#[cfg(not(feature = "withserde"))]
pub type Map<String, Value> = HashMap<String, Value>;

#[cfg(not(feature = "withserde"))]
#[derive(Clone, Debug, PartialEq)]
pub struct Number {
    n: N,
}

#[cfg(not(feature = "withserde"))]
#[derive(Clone, Debug, PartialEq)]
pub enum N {
    Float(f64),
}

#[cfg(not(feature = "withserde"))]
impl Number {
    pub fn from_f64(f: f64) -> Option<Number> {
        Some(Number { n: N::Float(f) })
    }
}

#[cfg(not(feature = "withserde"))]
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

#[cfg(not(feature = "withserde"))]
#[allow(unused_must_use)]
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::Object(ref obj) => {
                "{".fmt(f);
                for (n, prop) in obj.iter().enumerate() {
                    if n != 0 {
                        ",".fmt(f);
                    }
                    "\"".fmt(f);
                    prop.0.escape_default().fmt(f);
                    "\":".fmt(f);
                    prop.1.fmt(f);
                }
                "}".fmt(f);
                Result::Ok(())
            }
            Value::Array(ref arr) => {
                "[".fmt(f);
                for (n, item) in arr.iter().enumerate() {
                    if n != 0 {
                        ",".fmt(f);
                    }
                    item.fmt(f);
                }
                "]".fmt(f);
                Result::Ok(())
            }
            Value::String(ref string) => write!(f, "\"{}\"", string.escape_default()),
            Value::Number(ref number) => match number.n {
                N::Float(number) => write!(f, "{}", number.to_string()),
            },
            Value::Bool(boolean) => write!(f, "{}", boolean.to_string()),
            Value::Null => write!(f, "null"),
        }
    }
}

pub type ParseError = u32;
