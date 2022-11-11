#![warn(future_incompatible)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![forbid(overflowing_literals)]
#![doc(test(attr(warn(unused))))]

pub mod clang;
pub mod compilation_database;
pub mod index;
mod utility;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
