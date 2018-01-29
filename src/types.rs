use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Str(String),
    Num(f32),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
    Boolean(bool),
}

#[allow(unused_must_use)]
impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            JsonValue::Object(ref obj) => {
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
            JsonValue::Array(ref arr) => {
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
            JsonValue::Str(ref string) => write!(f, "\"{}\"", string.escape_default()),
            JsonValue::Num(number) => write!(f, "{}", number.to_string()),
            JsonValue::Boolean(boolean) => write!(f, "{}", boolean.to_string()),
        }
    }
}
pub type ParseError = u32;
