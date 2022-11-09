use std::hash::Hash;
use std::marker::PhantomData;
use std::path::Path;

use crate::clang::Clang;
use crate::utility::{cxstring_into_string, path_to_cstring};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ExcludePCH {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum DisplayDiagnostics {
    On,
    Off,
}

#[derive(Debug)]
pub struct Index<'clang> {
    raw: clang_sys::CXIndex,
    _clang: PhantomData<&'clang Clang>,
}

impl<'clang> Index<'clang> {
    fn new(exclude: ExcludePCH, display: DisplayDiagnostics) -> Self {
        let raw = unsafe {
            clang_sys::clang_createIndex(
                i32::from(exclude == ExcludePCH::On),
                i32::from(display == DisplayDiagnostics::On),
            )
        };
        assert!(!raw.is_null());
        Self {
            raw,
            _clang: PhantomData,
        }
    }

    pub fn with_display_diagnostics(_clang: &Clang) -> Self {
        Index::new(ExcludePCH::Off, DisplayDiagnostics::On)
    }
    pub fn with_exclude_pch(_clang: &Clang) -> Self {
        Index::new(ExcludePCH::On, DisplayDiagnostics::Off)
    }
    pub fn with_exclude_pch_and_display_diagnostics(_clang: &Clang) -> Self {
        Index::new(ExcludePCH::On, DisplayDiagnostics::On)
    }
    pub fn with(_clang: &Clang) -> Self {
        Index::new(ExcludePCH::Off, DisplayDiagnostics::Off)
    }
}

impl<'clang> Drop for Index<'clang> {
    fn drop(&mut self) {
        assert!(!self.raw.is_null());
        unsafe { clang_sys::clang_disposeIndex(self.raw) };
    }
}

#[derive(Debug)]
pub struct TranslationUnit<'index> {
    raw: clang_sys::CXTranslationUnit,
    _index: PhantomData<&'index Index<'index>>,
}

impl<'index> TranslationUnit<'index> {
    pub fn new<P: AsRef<Path>>(index: &'index Index<'index>, ast_filename: P) -> Self {
        let ast_filename = path_to_cstring(ast_filename);
        let raw =
            unsafe { clang_sys::clang_createTranslationUnit(index.raw, ast_filename.as_ptr()) };
        assert!(!raw.is_null());
        TranslationUnit {
            raw,
            _index: PhantomData,
        }
    }
    pub fn create_cursor(&self) -> Cursor<'_> {
        assert!(!self.raw.is_null());
        let raw = unsafe { clang_sys::clang_getTranslationUnitCursor(self.raw) };
        assert_eq!(unsafe { clang_sys::clang_Cursor_isNull(raw) }, 0);
        Cursor {
            raw,
            _tu: PhantomData,
        }
    }
}

impl<'index> Drop for TranslationUnit<'index> {
    fn drop(&mut self) {
        assert!(!self.raw.is_null());
        unsafe { clang_sys::clang_disposeTranslationUnit(self.raw) };
    }
}

#[derive(Debug)]
pub struct Cursor<'tu> {
    raw: clang_sys::CXCursor,
    _tu: PhantomData<&'tu TranslationUnit<'tu>>,
}

impl<'tu> Cursor<'tu> {
    fn from_raw(raw: clang_sys::CXCursor) -> Self {
        assert_eq!(unsafe { clang_sys::clang_Cursor_isNull(raw) }, 0);
        Self {
            raw,
            _tu: PhantomData,
        }
    }

    pub fn kind_spelling(&self) -> String {
        unsafe {
            let kind = clang_sys::clang_getCursorKind(self.raw);
            cxstring_into_string(clang_sys::clang_getCursorKindSpelling(kind))
        }
    }
    pub fn spelling(&self) -> String {
        unsafe { cxstring_into_string(clang_sys::clang_getCursorSpelling(self.raw)) }
    }
    pub fn is_from_main_file(&self) -> bool {
        unsafe {
            let location = clang_sys::clang_getCursorLocation(self.raw);
            clang_sys::clang_Location_isFromMainFile(location) == 0
        }
    }
}

pub type Payload = *const std::ffi::c_void;
pub fn to_payload<T>(v: &T) -> Payload {
    v as *const _ as Payload
}

/// convert payload to its original type
///
/// # Safety
///
/// It is undefined behavior if the wrong type `T` is given
///
/// # Example
///
/// ```
/// use clang_transformer::index::{to_payload, from_payload};
///
/// let i = 42_i32;
/// let payload = to_payload(&i);
/// let j = unsafe { &*(payload as *const i32) };
/// assert_eq!(&i as *const _, j as *const _);
/// assert_eq!(i, *j);
/// ```
pub unsafe fn from_payload<'a, T>(payload: Payload) -> &'a T {
    &*(payload as *const T)
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChildVisitResult {}

impl ChildVisitResult {
    pub const BREAK: clang_sys::CXChildVisitResult = clang_sys::CXChildVisit_Break;
    pub const CONTINUE: clang_sys::CXChildVisitResult = clang_sys::CXChildVisit_Continue;
    pub const RECURSIVE: clang_sys::CXChildVisitResult = clang_sys::CXChildVisit_Recurse;
}

pub fn visit_children<'tu, F>(cursor: &Cursor<'tu>, f: F, payload: Payload)
where
    F: Fn(&Cursor<'tu>, &Cursor<'tu>, Payload) -> i32,
{
    trait NodeCallback<'tu> {
        fn call(&self, cursor: &Cursor<'tu>, parent: &Cursor<'tu>, payload: Payload) -> i32;
    }
    impl<'tu, F> NodeCallback<'tu> for F
    where
        F: Fn(&Cursor<'tu>, &Cursor<'tu>, Payload) -> i32,
    {
        fn call(&self, cursor: &Cursor<'tu>, parent: &Cursor<'tu>, payload: Payload) -> i32 {
            self(cursor, parent, payload)
        }
    }
    extern "C" fn visitor(
        cursor: clang_sys::CXCursor,
        parent: clang_sys::CXCursor,
        data: clang_sys::CXClientData,
    ) -> clang_sys::CXChildVisitResult {
        let (f, payload) = unsafe { *(data as *mut (&dyn NodeCallback<'_>, Payload)) };
        f.call(
            &Cursor::from_raw(cursor),
            &Cursor::from_raw(parent),
            payload,
        )
    }
    let callback = &f as &dyn NodeCallback<'_>;
    let mut payload = (callback, payload);
    unsafe {
        clang_sys::clang_visitChildren(
            cursor.raw,
            visitor,
            &mut payload as *mut _ as clang_sys::CXClientData,
        )
    };
}
