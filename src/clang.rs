use std::ffi::c_void;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Clang(PhantomData<*const ()>);

impl Clang {
    pub fn new() -> Self {
        clang_sys::load().unwrap();
        Self(PhantomData)
    }
}

impl Default for Clang {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Clang {
    fn drop(&mut self) {
        clang_sys::unload().unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExcludePCH {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DisplayDiagnostics {
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
    pub fn new(index: &'index Index<'index>, ast_filename: &str) -> Self {
        let ast_filename = str_to_cstring(ast_filename);
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
}

pub fn visit_children<'tu, F>(cursor: &Cursor<'tu>, f: F)
where
    F: Fn(&Cursor<'tu>, &Cursor<'tu>) -> i32,
{
    trait NodeCallback<'tu> {
        fn call(&self, cursor: &Cursor<'tu>, parent: &Cursor<'tu>) -> i32;
    }
    impl<'tu, F> NodeCallback<'tu> for F
    where
        F: Fn(&Cursor<'tu>, &Cursor<'tu>) -> i32,
    {
        fn call(&self, cursor: &Cursor<'tu>, parent: &Cursor<'tu>) -> i32 {
            self(cursor, parent)
        }
    }
    extern "C" fn visitor(
        cursor: clang_sys::CXCursor,
        parent: clang_sys::CXCursor,
        data: clang_sys::CXClientData,
    ) -> clang_sys::CXChildVisitResult {
        let f = unsafe { *(data as *const &dyn NodeCallback<'_>) };
        f.call(&Cursor::from_raw(cursor), &Cursor::from_raw(parent))
    }
    let callback = &f as &dyn NodeCallback<'_>;
    unsafe {
        clang_sys::clang_visitChildren(cursor.raw, visitor, &callback as *const _ as *mut c_void)
    };
}

fn a_visitor<'tu>(cursor: &Cursor<'tu>, _parent: &Cursor<'tu>) -> i32 {
    unsafe {
        let location = clang_sys::clang_getCursorLocation(cursor.raw);
        if clang_sys::clang_Location_isFromMainFile(location) == 0 {
            return clang_sys::CXChildVisit_Continue;
        }
        let cursor_kind = clang_sys::clang_getCursorKind(cursor.raw);
        let cursor_kind_name = clang_sys::clang_getCursorKindSpelling(cursor_kind);
        let cursor_spelling = clang_sys::clang_getCursorSpelling(cursor.raw);
        println!(
            "-- {:?}\n   {:?}\n   {:?}",
            location,
            cxstring_into_string(cursor_kind_name),
            cxstring_into_string(cursor_spelling)
        );
        visit_children(cursor, a_visitor);
        clang_sys::CXChildVisit_Continue
    }
}

pub fn pay_a_visit(cursor: &Cursor<'_>) {
    visit_children(cursor, a_visitor)
}

/// convert a `CXString` to `String`
///
/// # SAFETY
///
/// CXString gets disposed inside this function
unsafe fn cxstring_into_string(cxstring: clang_sys::CXString) -> String {
    let s = std::ffi::CStr::from_ptr(clang_sys::clang_getCString(cxstring))
        .to_string_lossy()
        .into_owned();
    clang_sys::clang_disposeString(cxstring);
    s
}

/// convert a `&str` to `CString`
///
/// # PANICS
///
/// it panics if `s` cannot be converted
fn str_to_cstring(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).unwrap()
}
