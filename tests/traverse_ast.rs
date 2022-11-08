use clang_transformer::clang::{
    from_payload, to_payload, visit_children, Clang, Cursor, Index, Payload, TranslationUnit,
};

fn visitor<'tu>(cursor: &Cursor<'tu>, _parent: &Cursor<'tu>, payload: Payload) -> i32 {
    let payload = unsafe { from_payload::<AstDataPayload>(payload) };
    if cursor.is_from_main_file() {
        return clang_sys::CXChildVisit_Continue;
    }
    let cursor_kind_spelling = cursor.kind_spelling();
    let cursor_spelling = cursor.spelling();
    let new_buf = payload.borrow().buf.clone()
        + &format!(
            "{:-<width$} {} ({})\n",
            "",
            cursor_kind_spelling,
            cursor_spelling,
            width = payload.borrow().level as usize,
        );
    payload.borrow_mut().buf = new_buf;
    let children_payload = AstDataPayload::new(AstData {
        level: payload.borrow().level + 1,
        buf: String::new(),
    });
    visit_children(cursor, visitor, to_payload(&children_payload));
    payload.borrow_mut().buf += &children_payload.borrow().buf;
    clang_sys::CXChildVisit_Continue
}

#[derive(Debug, Default)]
struct AstData {
    level: i32,
    buf: String,
}
type AstDataPayload = std::cell::RefCell<AstData>;

fn collect_ast(cursor: &Cursor<'_>) -> String {
    let data = AstDataPayload::new(AstData {
        level: 0,
        buf: String::new(),
    });
    visit_children(cursor, visitor, to_payload(&data));
    data.take().buf
}

fn generate_ast(filename: &str) -> String {
    let ast_filename = "traverse_ast.ast";
    let status = std::process::Command::new("clang++")
        .arg("-emit-ast")
        .arg(filename)
        .status()
        .unwrap_or_else(|e| panic!("clang should generate .ast for {}, {}", filename, e));
    assert!(status.success());
    assert!(std::path::Path::new(ast_filename).exists());
    ast_filename.to_owned()
}

fn read_test_oracle(filename: &str) -> String {
    std::fs::read_to_string(filename).unwrap()
}

#[test]
fn it_works() {
    let ast_filename = generate_ast("tests/artifacts/traverse_ast.cpp");
    let oracle = read_test_oracle("tests/artifacts/traverse_ast.test_oracle");

    let clang = Clang::default();
    let index = Index::with_display_diagnostics(&clang);
    let tu = TranslationUnit::new(&index, &ast_filename);
    let cursor = tu.create_cursor();
    let buf = collect_ast(&cursor);

    assert_eq!(buf, oracle);
}