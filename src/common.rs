use std::num::ParseIntError;
use std::str::{self, FromStr};
use std::u32;

use nom::{digit, hex_digit};

named!(pub boolean<bool>,
    map!(
        alt_complete!(tag!("true") | tag!("false")),
        |value: &[u8]| value == b"true"
    )
);

named!(
    unsigned_float,
    recognize!(alt_complete!(
        delimited!(digit, tag!("."), opt!(complete!(digit)))
            | delimited!(opt!(digit), tag!("."), digit)
            | digit
    ))
);

named!(pub float<f32>, map_res!(
    map_res!(
        recognize!(alt_complete!(
            delimited!(
                pair!(opt!(alt!(tag!("+") | tag!("-"))), unsigned_float),
                tag!("e"),
                pair!(opt!(alt!(tag!("+") | tag!("-"))), unsigned_float)
            ) |
            unsigned_float
        )),
        str::from_utf8
    ),
    FromStr::from_str
));

fn to_i(i: &str) -> Result<u32, ParseIntError> {
    u32::from_str_radix(i, 16)
}

named!(pub int<u32>, map_res!(
    map_res!(
        preceded!(tag!("0x"), hex_digit),
        str::from_utf8
    ),
    to_i
));

// TODO: add support for octal
named!(pub number<f32>, alt_complete!(
    map!(int, |i| { i as f32 }) |
    float
));
