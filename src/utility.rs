use std::ffi::{CStr, CString};
use std::path::Path;

/// convert a `&str` to `CString`
///
/// # Panics
///
/// it panics if `s` cannot be converted
pub(crate) fn str_to_cstring(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub(crate) fn path_to_cstring<P: AsRef<Path>>(p: P) -> CString {
    str_to_cstring(p.as_ref().to_str().unwrap())
}

/// convert a `CXString` to `String`
///
/// # Safety
///
/// CXString gets disposed inside this function
pub(crate) unsafe fn cxstring_into_string(cxstring: clang_sys::CXString) -> String {
    let s = CStr::from_ptr(clang_sys::clang_getCString(cxstring))
        .to_string_lossy()
        .into_owned();
    clang_sys::clang_disposeString(cxstring);
    s
}

#[allow(dead_code)]
pub(crate) mod traits {
    pub(crate) fn is_small_value_struct<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
            + Clone
            + Default
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    pub(crate) fn is_small_value_enum<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    pub(crate) fn is_ffi_struct<T>(_: &T)
    where
        T: std::fmt::Debug,
    {
        // without calling fmt, `#[derive(Debug)]` may not be considered covered
        // but since we do not normally object properly for traits checking
        // some fields can be invalid (null), which causes crash when calling `fmt`
        // but adding conditional checks for `raw.is_null()` seems to be a safety
        // loop hold to me
        // so I'd rather choose lower coverage number.
    }
}
