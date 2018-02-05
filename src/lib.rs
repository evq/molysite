#![feature(str_escape)] 

#[macro_use]
extern crate nom;

#[cfg(feature="withserde")]
extern crate serde_json;

pub mod types;

#[macro_use]
mod common;
pub mod json;
pub mod hcl;
