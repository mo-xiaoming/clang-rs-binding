#![warn(future_incompatible)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![forbid(overflowing_literals)]

pub fn with_chdir<P: AsRef<std::path::Path>, R, F: Fn() -> R>(p: P, f: F) -> R {
    let current_dir = std::env::current_dir().unwrap();
    assert!(
        current_dir.starts_with("/"),
        "current dir {:?} is not a absolute path",
        current_dir
    );
    std::env::set_current_dir(p).unwrap();
    let r = f();
    std::env::set_current_dir(current_dir).unwrap();
    r
}

pub mod clang;
pub mod compilation_database;
pub mod index;
mod utility;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
