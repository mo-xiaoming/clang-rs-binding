//! ```cpp
//! template <typename T> bool f(T x) { return x % 2; }
//! ```
//!
//! ```text
//! $ clang -Xclang -ast-dump -fsyntax-only src/foo.cpp
//! TranslationUnitDecl 0x55cfadde8838 <<invalid sloc>> <invalid sloc>
//! |-TypedefDecl 0x55cfadde90a0 <<invalid sloc>> <invalid sloc> implicit __int128_t '__int128'
//! | `-BuiltinType 0x55cfadde8e00 '__int128'
//! |-TypedefDecl 0x55cfadde9110 <<invalid sloc>> <invalid sloc> implicit __uint128_t 'unsigned __int128'
//! | `-BuiltinType 0x55cfadde8e20 'unsigned __int128'
//! |-TypedefDecl 0x55cfadde9488 <<invalid sloc>> <invalid sloc> implicit __NSConstantString '__NSConstantString_tag'
//! | `-RecordType 0x55cfadde9200 '__NSConstantString_tag'
//! |   `-CXXRecord 0x55cfadde9168 '__NSConstantString_tag'
//! |-TypedefDecl 0x55cfadde9520 <<invalid sloc>> <invalid sloc> implicit __builtin_ms_va_list 'char *'
//! | `-PointerType 0x55cfadde94e0 'char *'
//! |   `-BuiltinType 0x55cfadde88e0 'char'
//! |-TypedefDecl 0x55cfade2ee68 <<invalid sloc>> <invalid sloc> implicit __builtin_va_list '__va_list_tag[1]'
//! | `-ConstantArrayType 0x55cfade2ee10 '__va_list_tag[1]' 1
//! |   `-RecordType 0x55cfadde9610 '__va_list_tag'
//! |     `-CXXRecord 0x55cfadde9578 '__va_list_tag'
//! `-FunctionTemplateDecl 0x55cfade2f128 <src/foo.cpp:1:1, col:51> col:28 f
//!   |-TemplateTypeParmDecl 0x55cfade2eec0 <col:11, col:20> col:20 referenced typename depth 0 index 0 T
//!   `-FunctionDecl 0x55cfade2f088 <col:23, col:51> col:28 f 'bool (T)'
//!     |-ParmVarDecl 0x55cfade2ef90 <col:30, col:32> col:32 referenced x 'T'
//!     `-CompoundStmt 0x55cfade2f2b8 <col:35, col:51>
//!       `-ReturnStmt 0x55cfade2f2a8 <col:37, col:48>
//!         `-BinaryOperator 0x55cfade2f288 <col:44, col:48> '<dependent type>' '%'
//!           |-DeclRefExpr 0x55cfade2f248 <col:44> 'T' lvalue ParmVar 0x55cfade2ef90 'x' 'T'
//!           `-IntegerLiteral 0x55cfade2f268 <col:48> 'int' 2
//! ```
//!
//! ```bash
//! $ clang++ -emit-ast src/foo.cpp
//! ```

use clang_transformer::clang::{pay_a_visit, Clang, Index, TranslationUnit};

fn main() {
    let clang = Clang::default();
    let index = Index::with_display_diagnostics(&clang);
    let tu = TranslationUnit::new(&index, "foo.ast");
    let cursor = tu.create_cursor();
    pay_a_visit(&cursor);
}
