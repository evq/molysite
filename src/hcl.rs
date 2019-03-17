use std::collections::HashMap;
use std::str::{self, FromStr};
use std::string::String;

use nom::IResult::Done;
use nom::{alphanumeric, eol, multispace, not_line_ending};

use common::{boolean, number};
use types::{JsonValue, ParseError};

pub fn parse_hcl(config: &str) -> Result<JsonValue, ParseError> {
    match hcl(&config.as_bytes()[..]) {
        Done(_, c) => Ok(c),
        _ => Err(0),
    }
}

named!(hcl<JsonValue>, map!(hcl_top, |h| JsonValue::Object(h)));

named!(end_of_line, alt!(eof!() | eol));

fn to_s(i: Vec<u8>) -> String {
    String::from_utf8_lossy(&i).into_owned()
}
fn slen(i: String) -> usize {
    i.len()
}
fn ulen(i: &[u8]) -> usize {
    i.len()
}
fn take_limited(min: usize, max: usize) -> usize {
    if max < min {
        return max;
    }
    return min;
}

named!(
    hcl_escaped_string<String>,
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
    hcl_template_string<String>,
    map!(
        do_parse!(tag!("${") >> s: take_until_and_consume!("}") >> (s)),
        |s| format!("${{{}}}", String::from_utf8_lossy(s))
    )
);

named!(
    hcl_quoted_escaped_string<String>,
    delimited!(
        tag!("\""),
        map!(
            fold_many0!(
                alt_complete!(
                    hcl_template_string
                        | flat_map!(
                            do_parse!(
                                max: map!(peek!(hcl_escaped_string), slen)
                                    >> min: map!(peek!(take_until!("${")), ulen)
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
            |s| s.join("")
        ),
        tag!("\"")
    )
);

named!(
    hcl_multiline_string<String>,
    map!(
        do_parse!(
            delimiter: tag!("<<")
                >> indent: opt!(tag!("-"))
                >> delimiter: terminated!(alphanumeric, eol)
                >> delimiter_str: expr_res!(str::from_utf8(delimiter))
                >> s: take_until!(delimiter_str)
                >> tag!(delimiter_str)
                >> end_of_line
                >> (indent, s)
        ),
        |(indent, s)| {
            let body = String::from_utf8_lossy(s);
            let lines: Vec<&str> = body.split("\n").collect();
            let mut out: Vec<&str> = Vec::new();
            let count = lines.len();

            let mut min_indent = 80;
            if let Some(_) = indent {
                for (i, line) in lines.clone().into_iter().enumerate() {
                    let indent_num = line.len() - line.trim_left().len();
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
named!(
    identifier_char,
    alt!(tag!("_") | tag!("-") | tag!(".") | alphanumeric)
);

named!(
    hcl_unquoted_key<String>,
    map!(
        fold_many0!(identifier_char, Vec::new(), |mut acc: Vec<_>, item| {
            acc.extend(item);
            acc
        }),
        to_s
    )
);

named!(
    hcl_quoted_escaped_key<String>,
    map!(
        do_parse!(tag!("\"") >> out: opt!(hcl_escaped_string) >> tag!("\"") >> (out)),
        |out| if let Some(val) = out {
            val
        } else {
            "".to_string()
        }
    )
);

named!(
    hcl_key<String>,
    alt!(hcl_quoted_escaped_key | hcl_unquoted_key)
);

named!(space, eat_separator!(&b" \t"[..]));

macro_rules! sp (
    ($i:expr, $($args:tt)*) => (
        {
            sep!($i, space, $($args)*)
        }
    )
);

named!(
    hcl_key_value<(String, JsonValue)>,
    sp!(alt_complete!(
        separated_pair!(hcl_key, tag!("="), hcl_value_nested_hash)
            | separated_pair!(hcl_key, tag!("="), hcl_value)
            | pair!(hcl_key, hcl_value_nested_hash)
    ))
);

named!(
    comment_one_line,
    do_parse!(alt!(tag!("//") | tag!("#")) >> opt!(not_line_ending) >> end_of_line >> (&b""[..]))
);

named!(
    comment_block,
    do_parse!(tag!("/*") >> take_until_and_consume!("*/") >> (&b""[..]))
);

named!(
    blanks,
    do_parse!(
        many0!(alt!(
            tag!(",") | multispace | comment_one_line | comment_block
        )) >> (&b""[..])
    )
);

named!(
    hcl_key_values<Vec<(String, JsonValue)>>,
    many0!(complete!(do_parse!(
        opt!(blanks) >> out: hcl_key_value >> opt!(blanks) >> (out)
    )))
);

named!(
    hcl_hash<HashMap<String, JsonValue>>,
    do_parse!(opt!(blanks) >> tag!("{") >> out: hcl_top >> tag!("}") >> opt!(blanks) >> (out))
);

named!(
    hcl_top<HashMap<String, JsonValue>>,
    map!(hcl_key_values, |tuple_vec| {
        let mut top: HashMap<String, JsonValue> = HashMap::new();
        for (k, v) in tuple_vec.into_iter().rev() {
            if top.contains_key(&k) {
                if let JsonValue::Array(ref v_a) = v {
                    if let Some(current) = top.remove(&k) {
                        if let JsonValue::Array(ref a) = current {
                            let mut copy = v_a.to_vec();
                            copy.extend(a.to_vec());
                            top.insert(k, JsonValue::Array(copy));
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

// a bit odd if you ask me
named!(
    hcl_value_nested_hash<JsonValue>,
    map!(
        // NOTE hcl allows arbitrarily deep nesting
        pair!(many0!(sp!(hcl_quoted_escaped_key)), hcl_value_hash),
        |(tuple_vec, value)| {
            let mut cur = value;
            for parent in tuple_vec.into_iter().rev() {
                let mut inner: Vec<JsonValue> = Vec::new();
                inner.push(cur);
                let mut h: HashMap<String, JsonValue> = HashMap::new();
                h.insert(parent.to_string(), JsonValue::Array(inner));
                cur = JsonValue::Object(h);
            }
            let mut outer: Vec<JsonValue> = Vec::new();
            outer.push(cur);
            JsonValue::Array(outer)
        }
    )
);

named!(
    hcl_value_hash<JsonValue>,
    map!(hcl_hash, |h| JsonValue::Object(h))
);

named!(
    hcl_array<Vec<JsonValue>>,
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

named!(
    hcl_value<JsonValue>,
    alt!(
        hcl_hash                    => { |h|   JsonValue::Object(h)            } |
        hcl_array                   => { |v|   JsonValue::Array(v)             } |
        hcl_quoted_escaped_string   => { |s|   JsonValue::Str(s) } |
        hcl_multiline_string        => { |s|   JsonValue::Str(s) } |
        number                      => { |num| JsonValue::Num(num)             } |
        boolean                     => { |b|   JsonValue::Boolean(b)           }
    )
);

#[test]
fn hcl_hex_num() {
    let test = "foo = 0x42";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Num(ref resp)) = dict.get("foo") {
            return assert_eq!(66., *resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_empty() {
    let test = "foo = \"\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo") {
            return assert_eq!("", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_escaped_quote_test() {
    let test = "foo = \"bar\\\"foo\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo") {
            return assert_eq!("bar\"foo", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_escaped_newline_test() {
    let test = "foo = \"bar\\nfoo\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo") {
            return assert_eq!("bar\nfoo", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_space_test() {
    let test = "foo = \"bar foo\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo") {
            return assert_eq!("bar foo", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_template_test() {
    let test = "foo = \"${bar\"foo}\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo") {
            return assert_eq!("${bar\"foo}", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_with_escapes_and_template_test() {
    let test = "foo = \"wow\\\"wow${bar\"foo}\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo") {
            return assert_eq!("wow\"wow${bar\"foo}", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_string_multi_with_template() {
    let test = "foo = \"wow\"\nbar= \"${bar\"foo}\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo") {
            return assert_eq!("wow", resp);
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_block_empty_key() {
    let test = "foo \"\" {\nbar = 1\n}";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Array(ref array)) = dict.get("foo") {
            if let Some(&JsonValue::Object(ref dict)) = array.get(0) {
                if let Some(&JsonValue::Array(ref array)) = dict.get("") {
                    if let Some(&JsonValue::Object(ref dict)) = array.get(0) {
                        if let Some(&JsonValue::Num(ref resp)) = dict.get("bar") {
                            return assert_eq!(1., *resp);
                        }
                    }
                }
            }
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_block_key() {
    let test = "potato \"salad\\\"is\" {\nnot = \"real\"\n}";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Array(ref array)) = dict.get("potato") {
            if let Some(&JsonValue::Object(ref dict)) = array.get(0) {
                if let Some(&JsonValue::Array(ref array)) = dict.get("salad\"is") {
                    if let Some(&JsonValue::Object(ref dict)) = array.get(0) {
                        if let Some(&JsonValue::Str(ref resp)) = dict.get("not") {
                            return assert_eq!("real", resp);
                        }
                    }
                }
            }
        }
    }
    panic!("object did not parse");
}

#[test]
fn hcl_block_nested_key() {
    let test = "potato \"salad\" \"is\" {\nnot = \"real\"\n}";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        println!("{:?}", dict);
        if let Some(&JsonValue::Array(ref array)) = dict.get("potato") {
            if let Some(&JsonValue::Object(ref dict)) = array.get(0) {
                if let Some(&JsonValue::Array(ref array)) = dict.get("salad") {
                    if let Some(&JsonValue::Object(ref dict)) = array.get(0) {
                        if let Some(&JsonValue::Array(ref array)) = dict.get("is") {
                            if let Some(&JsonValue::Object(ref dict)) = array.get(0) {
                                if let Some(&JsonValue::Str(ref resp)) = dict.get("not") {
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
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo_bar") {
            return assert_eq!("bar", resp);
        }
    }

    let test = "foo_bar = \"bar\"";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Str(ref resp)) = dict.get("foo_bar") {
            return assert_eq!("bar", resp);
        }
    }

    panic!("object did not parse");
}

#[test]
fn hcl_slice_expand() {
    let test = "service \"foo\" {
  key = \"value\"
}

service \"bar\" {
  key = \"value\"
}";
    if let Ok(JsonValue::Object(dict)) = parse_hcl(test) {
        if let Some(&JsonValue::Array(ref array)) = dict.get("service") {
            let mut pass = false;
            if let Some(&JsonValue::Object(_)) = array.get(0) {
                pass = true;
            }
            if !pass {
                panic!("missing nested object")
            }
            pass = false;
            if let Some(&JsonValue::Object(_)) = array.get(1) {
                pass = true;
            }
            if !pass {
                panic!("missing nested object")
            }
        }
    }
}
