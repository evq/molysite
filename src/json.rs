//! This was modified from https://github.com/Geal/nom/blob/master/tests/json.rs
//! Copyright (c) 2015-2016 Geoffroy Couprie - MIT License

use std::collections::HashMap;
use std::str;

use nom::IResult::Done;

use crate::common::{boolean, float};
use crate::types::{JsonValue, ParseError};

// NOTE this json parser is only included for internal verification purposes
// the standard hcl parser by hashicorp includes a nonstandrd json parser
// this is not intended to mirror that

pub fn parse_json(config: &str) -> Result<JsonValue, ParseError> {
    match json(&config.as_bytes()[..]) {
        Done(_, c) => Ok(c),
        _ => Err(0),
    }
}

named!(json<JsonValue>, map!(json_hash, |h| JsonValue::Object(h)));

fn to_s(i: Vec<u8>) -> String {
    String::from_utf8_lossy(&i).into_owned()
}

named!(
    json_escaped_string<String>,
    map!(
        escaped_transform!(
            is_not!("\\\"\n"),
            '\\',
            alt!(
                tag!("\\")       => { |_| &b"\\"[..] } |
                tag!("\"")       => { |_| &b"\""[..] } |
                tag!("n")        => { |_| &b"\n"[..] }
            )
        ),
        to_s
    )
);

named!(
    json_string<String>,
    delimited!(
        tag!("\""),
        map!(
            fold_many0!(json_escaped_string, Vec::new(), |mut acc: Vec<_>, item| {
                acc.push(item);
                acc
            }),
            |s| s.join("")
        ),
        tag!("\"")
    )
);

named!(
    json_array<Vec<JsonValue>>,
    ws!(delimited!(
        tag!("["),
        separated_list!(tag!(","), json_value),
        tag!("]")
    ))
);

named!(
    json_key_value<(String, JsonValue)>,
    ws!(separated_pair!(json_string, tag!(":"), json_value))
);

named!(
    json_hash<HashMap<String, JsonValue>>,
    ws!(map!(
        delimited!(
            tag!("{"),
            separated_list!(tag!(","), json_key_value),
            tag!("}")
        ),
        |tuple_vec| {
            let mut h: HashMap<String, JsonValue> = HashMap::new();
            for (k, v) in tuple_vec {
                h.insert(String::from(k), v);
            }
            h
        }
    ))
);

named!(
    json_value<JsonValue>,
    ws!(alt!(
        json_hash   => { |h|   JsonValue::Object(h)            } |
        json_array  => { |v|   JsonValue::Array(v)             } |
        json_string => { |s|   JsonValue::Str(String::from(s)) } |
        float       => { |num| JsonValue::Num(num)             } |
        boolean     => { |b|   JsonValue::Boolean(b)           }
    ))
);

#[test]
fn json_bool_test() {
    let test = "  { \"a\"\t: true,
  \"b\": \"false\"
  }";

    if let Ok(JsonValue::Object(dict)) = parse_json(test) {
        if let Some(&JsonValue::Boolean(ref resp)) = dict.get("a") {
            assert_eq!(true, *resp);
        }
        if let Some(&JsonValue::Boolean(ref resp)) = dict.get("b") {
            assert_eq!(false, *resp);
        }
        return;
    }
    panic!("object did not parse");
}

#[test]
fn json_hash_test() {
    let test = "  { \"a\"\t: 42,
  \"b\": \"x\"
  }";

    if let Ok(JsonValue::Object(dict)) = parse_json(test) {
        if let Some(&JsonValue::Num(ref resp)) = dict.get("a") {
            assert_eq!(42., *resp);
        }
        if let Some(&JsonValue::Str(ref resp)) = dict.get("b") {
            assert_eq!("x", *resp);
        }
        return;
    }
    panic!("object did not parse");
}

#[test]
fn json_parse_example_test() {
    let test = "  { \"a\"\t: 42,
  \"b\": [ \"x\", \"y\", 12 ] ,
  \"c\": { \"hello\" : \"world\"
  }
  }";

    if let Ok(JsonValue::Object(dict)) = parse_json(test) {
        if let Some(&JsonValue::Num(ref resp)) = dict.get("a") {
            assert_eq!(42., *resp);
        }
        if let Some(&JsonValue::Array(ref arr)) = dict.get("b") {
            if let Some(&JsonValue::Str(ref resp)) = arr.get(0) {
                assert_eq!("x", *resp);
            }
            if let Some(&JsonValue::Str(ref resp)) = arr.get(1) {
                assert_eq!("y", *resp);
            }
            if let Some(&JsonValue::Num(ref resp)) = arr.get(2) {
                assert_eq!(12., *resp);
            }
        }
        if let Some(&JsonValue::Object(ref dict)) = dict.get("c") {
            if let Some(&JsonValue::Str(ref resp)) = dict.get("hello") {
                assert_eq!("world", *resp);
            }
        }
        return;
    }
    panic!("object did not parse");
}
