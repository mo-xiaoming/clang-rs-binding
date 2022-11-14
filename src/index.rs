use std::hash::Hash;
use std::marker::PhantomData;
use std::path::Path;

use crate::clang::Clang;
//use crate::compilation_database::CompileCommand;
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

impl<'clang> Drop for Index<'clang> {
    fn drop(&mut self) {
        unsafe { clang_sys::clang_disposeIndex(self.raw) };
    }
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
}

impl Clang {
    pub fn create_index_with_display_diagnostics(&self) -> Index<'_> {
        Index::new(ExcludePCH::Off, DisplayDiagnostics::On)
    }
    pub fn create_index_with_exclude_pch(&self) -> Index<'_> {
        Index::new(ExcludePCH::On, DisplayDiagnostics::Off)
    }
    pub fn create_index_with_exclude_pch_and_display_diagnostics(&self) -> Index<'_> {
        Index::new(ExcludePCH::On, DisplayDiagnostics::On)
    }
    pub fn create_index(&self) -> Index<'_> {
        Index::new(ExcludePCH::Off, DisplayDiagnostics::Off)
    }
}

#[derive(Debug)]
pub struct TranslationUnit<'index> {
    raw: clang_sys::CXTranslationUnit,
    _index: PhantomData<&'index Index<'index>>,
}

impl<'index> Drop for TranslationUnit<'index> {
    fn drop(&mut self) {
        unsafe { clang_sys::clang_disposeTranslationUnit(self.raw) };
    }
}

impl<'index> TranslationUnit<'index> {
    fn from_raw(raw: clang_sys::CXTranslationUnit) -> Self {
        Self {
            raw,
            _index: PhantomData,
        }
    }
}

impl<'index> Index<'index> {
    pub fn create_translation_unit<P: AsRef<Path>>(&self, ast_filename: P) -> TranslationUnit<'_> {
        let ast_filename = path_to_cstring(ast_filename);
        let raw =
            unsafe { clang_sys::clang_createTranslationUnit(self.raw, ast_filename.as_ptr()) };
        assert!(!raw.is_null());
        TranslationUnit::from_raw(raw)
    }
    pub fn parse_translation_unit_from_compile_command(
        &self,
        compile_command: crate::compilation_database::CompileCommand,
    ) -> TranslationUnit<'_> {
        let raw = unsafe {
            let num_args = compile_command.get_num_args();
            let mut arguments = Vec::with_capacity(num_args as usize);
            let mut args = Vec::with_capacity(num_args as usize);
            for i in 0..num_args {
                let arg = clang_sys::clang_CompileCommand_getArg(compile_command.raw, i);
                args.push(arg);
                let arg = clang_sys::clang_getCString(arg);
                arguments.push(arg)
            }
            let raw = clang_sys::clang_parseTranslationUnit(
                self.raw,
                std::ptr::null(),
                arguments.as_ptr(),
                num_args as i32,
                std::ptr::null_mut(),
                0,
                clang_sys::CXTranslationUnit_None,
            );
            assert!(!raw.is_null());
            args.iter().for_each(|e| clang_sys::clang_disposeString(*e));
            raw
        };
        assert!(!raw.is_null());
        TranslationUnit::from_raw(raw)
    }
}

#[derive(Debug)]
pub struct Cursor<'tu> {
    raw: clang_sys::CXCursor,
    _tu: PhantomData<&'tu TranslationUnit<'tu>>,
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
/// use clang_rs_binding::index::{to_payload, from_payload};
///
/// let i = 42_i32;
/// let payload = to_payload(&i);
/// let j = unsafe { &*from_payload(payload) };
/// assert_eq!(&i as *const _, j as *const _);
/// assert_eq!(i, *j);
/// ```
pub unsafe fn from_payload<'a, T>(payload: Payload) -> &'a T {
    &*(payload as *const T)
}

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChildVisitResult {}

impl ChildVisitResult {
    pub const BREAK: clang_sys::CXChildVisitResult = clang_sys::CXChildVisit_Break;
    pub const CONTINUE: clang_sys::CXChildVisitResult = clang_sys::CXChildVisit_Continue;
    pub const RECURSIVE: clang_sys::CXChildVisitResult = clang_sys::CXChildVisit_Recurse;
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
    pub fn is_function_decl(&self) -> bool {
        unsafe { clang_sys::CXCursor_FunctionDecl == clang_sys::clang_getCursorKind(self.raw) }
    }
    pub fn is_cxx_method(&self) -> bool {
        unsafe { clang_sys::CXCursor_CXXMethod == clang_sys::clang_getCursorKind(self.raw) }
    }
    pub fn is_function_template(&self) -> bool {
        unsafe { clang_sys::CXCursor_FunctionTemplate == clang_sys::clang_getCursorKind(self.raw) }
    }
    pub fn extent(&self) -> SourceRange {
        SourceRange {
            raw: unsafe { clang_sys::clang_getCursorExtent(self.raw) },
            _cursor: PhantomData,
        }
    }
    pub fn visit_children<F>(&self, f: F, payload: Payload)
    where
        F: Fn(&Cursor, &Cursor, Payload) -> i32,
    {
        trait NodeCallback {
            fn call(&self, cursor: &Cursor, parent: &Cursor, payload: Payload) -> i32;
        }
        impl<F> NodeCallback for F
        where
            F: Fn(&Cursor, &Cursor, Payload) -> i32,
        {
            fn call(&self, cursor: &Cursor, parent: &Cursor, payload: Payload) -> i32 {
                self(cursor, parent, payload)
            }
        }
        extern "C" fn visitor(
            cursor: clang_sys::CXCursor,
            parent: clang_sys::CXCursor,
            data: clang_sys::CXClientData,
        ) -> clang_sys::CXChildVisitResult {
            let (f, payload) = unsafe { *(data as *mut (&dyn NodeCallback, Payload)) };
            f.call(
                &Cursor::from_raw(cursor),
                &Cursor::from_raw(parent),
                payload,
            )
        }
        let callback = &f as &dyn NodeCallback;
        let mut payload = (callback, payload);
        unsafe {
            clang_sys::clang_visitChildren(
                self.raw,
                visitor,
                &mut payload as *mut _ as clang_sys::CXClientData,
            )
        };
    }
}

impl<'index> TranslationUnit<'index> {
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

#[derive(Debug)]
pub struct SourceRange<'cursor> {
    raw: clang_sys::CXSourceRange,
    _cursor: PhantomData<&'cursor Cursor<'cursor>>,
}

impl<'cursor> SourceRange<'cursor> {
    pub fn start(&self) -> SourceLocation<'_> {
        unsafe { SourceLocation::from_raw(clang_sys::clang_getRangeStart(self.raw)) }
    }
    pub fn end(&self) -> SourceLocation<'_> {
        unsafe { SourceLocation::from_raw(clang_sys::clang_getRangeEnd(self.raw)) }
    }
}

#[derive(Debug)]
pub struct File {
    raw: clang_sys::CXFile,
}

impl File {
    pub(crate) fn from_raw(raw: clang_sys::CXFile) -> Self {
        Self { raw }
    }
}

#[derive(Debug)]
pub struct SourceLocation<'source_range> {
    raw: clang_sys::CXSourceLocation,
    _source_range: PhantomData<&'source_range SourceRange<'source_range>>,
}

pub struct SpellingLocation {
    file: File,
    line: u32,
    column: u32,
    offset: u32,
}

impl std::fmt::Debug for SpellingLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let filename = unsafe { cxstring_into_string(clang_sys::clang_getFileName(self.file.raw)) };
        f.debug_struct("SpellingLocation")
            .field("file", &filename)
            .field("line", &self.line)
            .field("column", &self.column)
            .field("offset", &self.offset)
            .finish()
    }
}

impl<'source_range> SourceLocation<'source_range> {
    pub(crate) fn from_raw(raw: clang_sys::CXSourceLocation) -> Self {
        Self {
            raw,
            _source_range: PhantomData,
        }
    }
    pub fn spelling_location(&self) -> SpellingLocation {
        let mut file = std::ptr::null_mut();
        let mut line = 0;
        let mut column = 0;
        let mut offset = 0;
        unsafe {
            clang_sys::clang_getSpellingLocation(
                self.raw,
                &mut file,
                &mut line,
                &mut column,
                &mut offset,
            );
        }
        SpellingLocation {
            file: File::from_raw(file),
            line,
            column,
            offset,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        let _clang = Clang::new();

        let exclude_pch = ExcludePCH::On;
        is_small_value_enum(&exclude_pch);

        let display_diag = DisplayDiagnostics::On;
        is_small_value_enum(&display_diag);

        let index = Index {
            raw: std::ptr::null_mut() as clang_sys::CXIndex,
            _clang: PhantomData,
        };
        is_ffi_struct(&index);

        let tu = TranslationUnit {
            raw: std::ptr::null_mut() as clang_sys::CXTranslationUnit,
            _index: PhantomData,
        };
        is_ffi_struct(&tu);

        let cursor = Cursor {
            raw: clang_sys::CXCursor::default(),
            _tu: PhantomData,
        };
        is_ffi_struct(&cursor);

        let child_visit_result = ChildVisitResult {};
        is_small_value_struct(&child_visit_result);

        let source_range = SourceRange {
            raw: clang_sys::CXSourceRange::default(),
            _cursor: PhantomData,
        };
        is_ffi_struct(&source_range);

        let source_location = SourceLocation {
            raw: clang_sys::CXSourceLocation::default(),
            _source_range: PhantomData,
        };
        is_ffi_struct(&source_location);

        let file = File {
            raw: std::ptr::null_mut() as clang_sys::CXFile,
        };
        is_ffi_struct(&file);

        let spelling_location = SpellingLocation {
            file: File {
                raw: std::ptr::null_mut() as clang_sys::CXFile,
            },
            line: 0,
            column: 0,
            offset: 0,
        };
        is_ffi_struct(&spelling_location);
    }
}
