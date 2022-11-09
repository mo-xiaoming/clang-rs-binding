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
