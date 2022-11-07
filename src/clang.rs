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

type Payload = *mut c_void;

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

fn a_visitor<'tu>(cursor: &Cursor<'tu>, _parent: &Cursor<'tu>, level: Payload) -> i32 {
    unsafe {
        let level = &mut *(level as *mut i32);
        let location = clang_sys::clang_getCursorLocation(cursor.raw);
        if clang_sys::clang_Location_isFromMainFile(location) == 0 {
            return clang_sys::CXChildVisit_Continue;
        }
        let cursor_kind = clang_sys::clang_getCursorKind(cursor.raw);
        let cursor_kind_name = clang_sys::clang_getCursorKindSpelling(cursor_kind);
        let cursor_spelling = clang_sys::clang_getCursorSpelling(cursor.raw);
        print!("{:-<width$}", '-', width = *level as usize);
        println!(
            " {:?} {} {}",
            location,
            cxstring_into_string(cursor_kind_name),
            cxstring_into_string(cursor_spelling)
        );
        *level += 1;
        visit_children(cursor, a_visitor, level as *mut _ as Payload);
        clang_sys::CXChildVisit_Continue
    }
}

/// should be same as
/// ```bash
/// clang -Xclang -ast-dump -fsyntax-only src/foo.cpp
/// ```
/// ```text
/// `-FunctionTemplateDecl 0x55b04d8dd098 <src/foo.cpp:1:1, line:4:1> line:2:6 f
///  |-TemplateTypeParmDecl 0x55b04d8dce30 <line:1:11, col:20> col:20 referenced typename depth 0 index 0 T
///  `-FunctionDecl 0x55b04d8dcff8 <line:2:1, line:4:1> line:2:6 f 'bool (T)'
///    |-ParmVarDecl 0x55b04d8dcf00 <col:8, col:10> col:10 referenced x 'T'
///    `-CompoundStmt 0x55b04d8dd228 <col:13, line:4:1>
///      `-ReturnStmt 0x55b04d8dd218 <line:3:3, col:14>
///        `-BinaryOperator 0x55b04d8dd1f8 <col:10, col:14> '<dependent type>' '%'
///          |-DeclRefExpr 0x55b04d8dd1b8 <col:10> 'T' lvalue ParmVar 0x55b04d8dcf00 'x' 'T'
///          `-IntegerLiteral 0x55b04d8dd1d8 <col:14> 'int' 2
/// ```
pub fn pay_a_visit(cursor: &Cursor<'_>) {
    let mut i = 1;
    visit_children(cursor, a_visitor, &mut i as *mut _ as Payload);
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
