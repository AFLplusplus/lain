//! This crate provides functionality one may find useful while developing a fuzzer. A recent
//! nightly Rust build is required for the specialization feature.
//!
//! Please consider this crate in "beta" and subject to breaking changes for minor version releases for pre-1.0.

#![feature(specialization)]

extern crate num;
extern crate num_derive;
extern crate num_traits;
extern crate self as lain;

pub extern crate byteorder;
pub extern crate field_offset;
pub extern crate lain_derive;
pub extern crate lazy_static;
pub extern crate rand;

pub use lain_derive::*;

#[macro_use]
pub extern crate log;

#[doc(hidden)]
pub mod buffer;
#[doc(hidden)]
pub mod dangerous_numbers;
pub mod driver;
#[doc(hidden)]
pub mod mutatable;
pub mod mutator;
#[doc(hidden)]
pub mod new_fuzzed;
pub mod prelude;
pub mod traits;
pub mod types;

pub fn hexdump(data: &[u8]) -> String {
    let mut ret = "------".to_string();
    for i in 0..16 {
        ret += &format!("{:02X} ", i);
    }

    let mut ascii = String::new();
    for (i, b) in data.iter().enumerate() {
        if i % 16 == 0 {
            ret += &format!("\t{}", ascii);
            ascii.clear();
            ret += &format!("\n{:04X}:", i);
        }

        ret += &format!(" {:02X}", b);
        // this is the printable ASCII range
        if *b >= 0x20 && *b <= 0x7f {
            ascii.push(*b as char);
        } else {
            ascii.push('.');
        }
    }

    if data.len() % 16 != 0 {
        for _i in 0..16 - (data.len() % 16) {
            ret += " ";
        }
    }

    ret += &format!("\t{}", ascii);
    ascii.clear();

    ret
}
