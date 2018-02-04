use std::num::ParseIntError;
use std::u32;

use nom;
use nom::{digit, hex_digit};
use nom::types::CompleteStr;

#[macro_export]
macro_rules! complete_named (
  ($name:ident, $submac:ident!( $($args:tt)* )) => (
    fn $name<'a>( i: CompleteStr<'a> ) -> nom::IResult<CompleteStr<'a>, CompleteStr<'a>, u32> {
      $submac!(i, $($args)*)
    }
  );
  ($name:ident<$o:ty>, $submac:ident!( $($args:tt)* )) => (
    fn $name<'a>( i: CompleteStr<'a> ) -> nom::IResult<CompleteStr<'a>, $o, u32> {
      $submac!(i, $($args)*)
    }
  );
  (pub $name:ident, $submac:ident!( $($args:tt)* )) => (
    pub fn $name<'a>( i: CompleteStr<'a> ) -> nom::IResult<CompleteStr<'a>, CompleteStr<'a>, u32> {
      $submac!(i, $($args)*)
    }
  );
  (pub $name:ident<$o:ty>, $submac:ident!( $($args:tt)* )) => (
    pub fn $name<'a>( i: CompleteStr<'a> ) -> nom::IResult<CompleteStr<'a>, $o, u32> {
      $submac!(i, $($args)*)
    }
  );
);

complete_named!(pub boolean<bool>,map!(
    alt!(tag!("true") | tag!("false")),
    |value: CompleteStr| value == CompleteStr("true")
));

complete_named!(unsigned_float, recognize!(alt_complete!(
    delimited!(digit, tag!("."), opt!(complete!(digit))) |
    delimited!(opt!(digit), tag!("."), digit) | 
    digit
)));

complete_named!(pub float<f32>, flat_map!(
    recognize!(alt_complete!(
        delimited!(
            pair!(opt!(alt!(tag!("+") | tag!("-"))), unsigned_float),
            tag!("e"),
            pair!(opt!(alt!(tag!("+") | tag!("-"))), unsigned_float)
        ) |
        unsigned_float
    )),
    parse_to!(f32)
));

fn complete_to_i(i: CompleteStr) -> Result<u32, ParseIntError> { 
    u32::from_str_radix(i.0, 16) 
}

complete_named!(int<u32>, map_res!(
    preceded!(tag!("0x"), hex_digit),
    complete_to_i
));
        
// TODO: add support for octal
complete_named!(pub number<f32>, alt_complete!(
    map!(int, |i| { i as f32 }) |
    float
));
