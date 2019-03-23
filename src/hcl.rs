use std::str;
use std::string::String;

use nom;
use nom::types::CompleteStr;
use nom::{alphanumeric, eol, multispace, not_line_ending, ExtendInto};

use crate::common::{boolean, number};
use crate::types::{Map, Number, ParseError, Value};

pub fn parse_hcl(config: &str) -> Result<Value, ParseError> {
    match hcl(CompleteStr(config)) {
        Ok((_, c)) => Ok(c),
        _ => Err(0),
    }
}

complete_named!(hcl<Value>, map!(hcl_top, |h| Value::Object(h)));

complete_named!(end_of_line, alt!(eof!() | eol));

fn slen(i: String) -> usize {
    i.len()
}
fn cslen(i: CompleteStr) -> usize {
    i.0.len()
}
fn take_limited(min: usize, max: usize) -> usize {
    if max < min {
        return max;
    }
    return min;
}

complete_named!(
    hcl_escaped_string<String>,
    escaped_transform!(
        is_not!("\\\"\n"),
        '\\',
        alt!(
            tag!("\\")       => { |_| "\\" } |
            tag!("\"")       => { |_| "\"" } |
            tag!("n")        => { |_| "\n" }
        )
    )
);

complete_named!(
    hcl_template_string<String>,
    map!(
        do_parse!(tag!("${") >> s: take_until_and_consume!("}") >> (s)),
        |s: CompleteStr| { format!("${{{}}}", s.0) }
    )
);

complete_named!(
    hcl_quoted_escaped_string<String>,
    delimited!(
        tag!("\""),
        map!(
            fold_many0!(
                alt!(
                    hcl_template_string
                        | flat_map!(
                            do_parse!(
                                max: map!(peek!(hcl_escaped_string), slen)
                                    >> min: map!(peek!(take_until!("${")), cslen)
                                    >> buf: take!(take_limited(min, max))
                                    >> (buf)
                            ),
                            hcl_escaped_string
                        )
                        | hcl_escaped_string
                ),
                Vec::new(),
                |mut acc: Vec<_>, item| {
                    acc.push(item);
                    acc
                }
            ),
            |s| { s.join("") }
        ),
        tag!("\"")
    )
);

complete_named!(
    hcl_multiline_string<String>,
    map!(
        do_parse!(
            tag!("<<")
                >> indent: opt!(tag!("-"))
                >> delimiter: terminated!(alphanumeric, eol)
                >> s: take_until!(delimiter.0)
                >> tag!(delimiter.0)
                >> end_of_line
                >> (indent, s)
        ),
        |(indent, s)| {
            let lines: Vec<&str> = s.0.split("\n").collect();
            let mut out: Vec<&str> = Vec::new();
            let count = lines.len();

            let mut min_indent = 80;
            if let Some(_) = indent {
                for (i, line) in lines.clone().into_iter().enumerate() {
                    let indent_num = line.len() - line.trim_start().len();
                    if indent_num < min_indent {
                        min_indent = indent_num;
                    }
                    if i != count - 1 {
                        if min_indent < indent_num {
                            // NOTE this behavior is odd, and will change based on the hcl2 specs
                            min_indent = 0;
                        }
                    }
                }
            }
            for (i, line) in lines.into_iter().enumerate() {
                if i != count - 1 {
                    if let Some(_) = indent {
                        out.push(&line[min_indent..])
                    } else {
                        out.push(line)
                    }
                }
            }
            out.join("\n") + "\n"
        }
    )
);

// close enough...
complete_named!(
    identifier_char,
    alt!(tag!("_") | tag!("-") | tag!(".") | alphanumeric)
);

complete_named!(
    hcl_unquoted_key<String>,
    fold_many0!(
        identifier_char,
        String::new(),
        |mut acc: String, item: CompleteStr| {
            //acc.extend(item);
            item.extend_into(&mut acc);
            acc
        }
    )
);

complete_named!(
    hcl_quoted_escaped_key<String>,
    map!(
        do_parse!(tag!("\"") >> out: opt!(hcl_escaped_string) >> tag!("\"") >> (out)),
        |out| {
            if let Some(val) = out {
                val
            } else {
                "".to_string()
            }
        }
    )
);

complete_named!(
    hcl_key<String>,
    alt!(hcl_quoted_escaped_key | hcl_unquoted_key)
);

complete_named!(space, eat_separator!(&b" \t"[..]));

macro_rules! sp (
    ($i:expr, $($args:tt)*) => (
        {
            sep!($i, space, $($args)*)
        }
    )
);

complete_named!(
    hcl_key_value<(String, Value)>,
    sp!(alt!(
        separated_pair!(hcl_key, tag!("="), hcl_value_nested_hash)
            | separated_pair!(hcl_key, tag!("="), hcl_value)
            | pair!(hcl_key, hcl_value_nested_hash)
    ))
);

complete_named!(
    comment_one_line,
    do_parse!(
        alt!(tag!("//") | tag!("#")) >> opt!(not_line_ending) >> end_of_line >> (CompleteStr(""))
    )
);

complete_named!(
    comment_block,
    do_parse!(tag!("/*") >> take_until_and_consume!("*/") >> (CompleteStr("")))
);

complete_named!(
    blanks,
    do_parse!(
        many0!(alt!(
            tag!(",") | multispace | comment_one_line | comment_block
        )) >> (CompleteStr(""))
    )
);

complete_named!(
    hcl_key_values<Vec<(String, Value)>>,
    many0!(do_parse!(
        opt!(blanks) >> out: hcl_key_value >> opt!(blanks) >> (out)
    ))
);

complete_named!(
    hcl_hash<Map<String, Value>>,
    do_parse!(opt!(blanks) >> tag!("{") >> out: hcl_top >> tag!("}") >> opt!(blanks) >> (out))
);

#[cfg(not(feature = "arraynested"))]
complete_named!(
    hcl_top<Map<String, Value>>,
    map!(hcl_key_values, |tuple_vec| {
        let mut top: Map<String, Value> = Map::new();
        for (k, v) in tuple_vec.into_iter().rev() {
            // FIXME use deep merge
            if top.contains_key(&k) {
                if let Value::Object(ref val_dict) = v {
                    if let Some(mut current) = top.remove(&k) {
                        if let Value::Object(ref mut top_dict) = current {
                            let mut copy = top_dict.clone();
                            let val_copy = val_dict.clone();
                            copy.extend(val_copy);
                            top.insert(k, Value::Object(copy));
                            continue;
                        } else {
                            top.insert(k, current);
                            continue;
                        }
                    }
                }
            }
            top.insert(k, v);
        }
        top
    })
);

#[cfg(feature = "arraynested")]
complete_named!(
    hcl_top<Map<String, Value>>,
    map!(hcl_key_values, |tuple_vec| {
        let mut top: Map<String, Value> = Map::new();
        for (k, v) in tuple_vec.into_iter().rev() {
            if top.contains_key(&k) {
                if let Value::Array(ref v_a) = v {
                    if let Some(current) = top.remove(&k) {
                        if let Value::Array(ref a) = current {
                            let mut copy = v_a.to_vec();
                            copy.extend(a.to_vec());
                            top.insert(k, Value::Array(copy));
                            continue;
                        }
                    }
                }
            }
            top.insert(k, v);
        }
        top
    })
);

#[cfg(not(feature = "arraynested"))]
complete_named!(
    hcl_value_nested_hash<Value>,
    map!(
        // NOTE hcl allows arbitrarily deep nesting
        pair!(many0!(sp!(hcl_quoted_escaped_key)), hcl_value_hash),
        |(tuple_vec, value)| {
            let mut cur = value;
            for parent in tuple_vec.into_iter().rev() {
                let mut h: Map<String, Value> = Map::new();
                h.insert(parent.to_string(), cur);
                cur = Value::Object(h);
            }
            cur
        }
    )
);

// a bit odd if you ask me
#[cfg(feature = "arraynested")]
complete_named!(
    hcl_value_nested_hash<Value>,
    map!(
        // NOTE hcl allows arbitrarily deep nesting
        pair!(many0!(sp!(hcl_quoted_escaped_key)), hcl_value_hash),
        |(tuple_vec, value)| {
            let mut cur = value;
            for parent in tuple_vec.into_iter().rev() {
                let mut inner: Vec<Value> = Vec::new();
                inner.push(cur);
                let mut h: Map<String, Value> = Map::new();
                h.insert(parent.to_string(), Value::Array(inner));
                cur = Value::Object(h);
            }
            let mut outer: Vec<Value> = Vec::new();
            outer.push(cur);
            Value::Array(outer)
        }
    )
);

complete_named!(hcl_value_hash<Value>, map!(hcl_hash, |h| Value::Object(h)));

complete_named!(
    hcl_array<Vec<Value>>,
    delimited!(
        tag!("["),
        do_parse!(
            init: fold_many0!(
                do_parse!(
                    opt!(blanks)
                        >> out: hcl_value
                        >> opt!(blanks)
                        >> tag!(",")
                        >> opt!(blanks)
                        >> (out)
                ),
                Vec::new(),
                |mut acc: Vec<_>, item| {
                    acc.push(item);
                    acc
                }
            ) >> ret: fold_many0!(
                do_parse!(opt!(blanks) >> out: hcl_value >> opt!(blanks) >> (out)),
                init,
                |mut acc: Vec<_>, item| {
                    acc.push(item);
                    acc
                }
            ) >> (ret)
        ),
        tag!("]")
    )
);

complete_named!(
    hcl_value<Value>,
    alt!(
        hcl_hash                    => { |h|   Value::Object(h)            } |
        hcl_array                   => { |v|   Value::Array(v)             } |
        hcl_quoted_escaped_string   => { |s|   Value::String(s) } |
        hcl_multiline_string        => { |s|   Value::String(s) } |
        number                      => { |num| Value::Number(Number::from_f64(num as f64).unwrap()) } |
        boolean                     => { |b|   Value::Bool(b)           }
    )
);

#[test]
fn hcl_hex_num() {
    let test = "foo = 0x42";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Number(ref resp)) = dict.get("foo") {
            return assert_eq!(Number::from_f64(66.).unwrap(), *resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_empty() {
    let test = "foo = \"\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo") {
            return assert_eq!("", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_escaped_quote_test() {
    let test = "foo = \"bar\\\"foo\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo") {
            return assert_eq!("bar\"foo", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_escaped_newline_test() {
    let test = "foo = \"bar\\nfoo\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo") {
            return assert_eq!("bar\nfoo", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_space_test() {
    let test = "foo = \"bar foo\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo") {
            return assert_eq!("bar foo", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_template_test() {
    let test = "foo = \"${bar\"foo}\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo") {
            return assert_eq!("${bar\"foo}", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_escapes_and_template_test() {
    let test = "foo = \"wow\\\"wow${bar\"foo}\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo") {
            return assert_eq!("wow\"wow${bar\"foo}", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_multi_with_template() {
    let test = "foo = \"wow\"\nbar= \"${bar\"foo}\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo") {
            return assert_eq!("wow", resp);
        }
    }
    panic!("object did not parse");
}

#[cfg(not(feature = "arraynested"))]
#[test]
fn hcl_block_empty_key() {
    let test = "foo \"\" {\nbar = 1\n}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Object(ref dict)) = dict.get("foo") {
            if let Some(&Value::Object(ref dict)) = dict.get("") {
                if let Some(&Value::Number(ref resp)) = dict.get("bar") {
                    return assert_eq!(Number::from_f64(1.).unwrap(), *resp);
                }
            }
        }
    }
    panic!("object did not parse");
}

#[cfg(feature = "arraynested")]
#[test]
fn hcl_block_empty_key() {
    let test = "foo \"\" {\nbar = 1\n}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Array(ref array)) = dict.get("foo") {
            if let Some(&Value::Object(ref dict)) = array.get(0) {
                if let Some(&Value::Array(ref array)) = dict.get("") {
                    if let Some(&Value::Object(ref dict)) = array.get(0) {
                        if let Some(&Value::Number(ref resp)) = dict.get("bar") {
                            return assert_eq!(Number::from_f64(1.).unwrap(), *resp);
                        }
                    }
                }
            }
        }
    }
    panic!("object did not parse");
}

#[cfg(not(feature = "arraynested"))]
#[test]
fn hcl_block_key() {
    let test = "potato \"salad\\\"is\" {\nnot = \"real\"\n}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Object(ref dict)) = dict.get("potato") {
            if let Some(&Value::Object(ref dict)) = dict.get("salad\"is") {
                if let Some(&Value::String(ref resp)) = dict.get("not") {
                    return assert_eq!("real", resp);
                }
            }
        }
    }
    panic!("object did not parse");
}

#[cfg(feature = "arraynested")]
#[test]
fn hcl_block_key() {
    let test = "potato \"salad\\\"is\" {\nnot = \"real\"\n}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Array(ref array)) = dict.get("potato") {
            if let Some(&Value::Object(ref dict)) = array.get(0) {
                if let Some(&Value::Array(ref array)) = dict.get("salad\"is") {
                    if let Some(&Value::Object(ref dict)) = array.get(0) {
                        if let Some(&Value::String(ref resp)) = dict.get("not") {
                            return assert_eq!("real", resp);
                        }
                    }
                }
            }
        }
    }
    panic!("object did not parse");
}

#[cfg(not(feature = "arraynested"))]
#[test]
fn hcl_block_nested_key() {
    let test = "potato \"salad\" \"is\" {\nnot = \"real\"\n}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Object(ref dict)) = dict.get("potato") {
            if let Some(&Value::Object(ref dict)) = dict.get("salad") {
                if let Some(&Value::Object(ref dict)) = dict.get("is") {
                    if let Some(&Value::String(ref resp)) = dict.get("not") {
                        return assert_eq!("real", resp);
                    }
                }
            }
        }
    }
    panic!("object did not parse");
}

#[cfg(feature = "arraynested")]
#[test]
fn hcl_block_nested_key() {
    let test = "potato \"salad\" \"is\" {\nnot = \"real\"\n}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Array(ref array)) = dict.get("potato") {
            if let Some(&Value::Object(ref dict)) = array.get(0) {
                if let Some(&Value::Array(ref array)) = dict.get("salad") {
                    if let Some(&Value::Object(ref dict)) = array.get(0) {
                        if let Some(&Value::Array(ref array)) = dict.get("is") {
                            if let Some(&Value::Object(ref dict)) = array.get(0) {
                                if let Some(&Value::String(ref resp)) = dict.get("not") {
                                    return assert_eq!("real", resp);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_key_chars() {
    let test = "foo_bar = \"bar\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo_bar") {
            return assert_eq!("bar", resp);
        }
    }

    let test = "foo_bar = \"bar\"";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::String(ref resp)) = dict.get("foo_bar") {
            return assert_eq!("bar", resp);
        }
    }

    panic!("object did not parse");
}

#[cfg(not(feature = "arraynested"))]
#[test]
fn hcl_slice_expand() {
    let test = "service \"foo\" {
  key = \"value\"
}

service \"bar\" {
  key = \"value\"
}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Object(ref dict)) = dict.get("service") {
            let mut pass = false;
            if let Some(&Value::Object(_)) = dict.get("foo") {
                pass = true;
            }
            if !pass {
                panic!("missing nested object")
            }
            pass = false;
            if let Some(&Value::Object(_)) = dict.get("bar") {
                pass = true;
            }
            if !pass {
                panic!("missing nested object")
            }
            return;
        }
    }
    panic!("object did not parse")
}

#[cfg(feature = "arraynested")]
#[test]
fn hcl_slice_expand() {
    let test = "service \"foo\" {
  key = \"value\"
}

service \"bar\" {
  key = \"value\"
}";
    if let Ok(Value::Object(dict)) = parse_hcl(test) {
        if let Some(&Value::Array(ref array)) = dict.get("service") {
            let mut pass = false;
            if let Some(&Value::Object(_)) = array.get(0) {
                pass = true;
            }
            if !pass {
                panic!("missing nested object")
            }
            pass = false;
            if let Some(&Value::Object(_)) = array.get(1) {
                pass = true;
            }
            if !pass {
                panic!("missing nested object")
            }
            return;
        }
    }
    panic!("object did not parse")
}
