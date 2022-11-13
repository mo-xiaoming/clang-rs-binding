use clang_rs_binding::clang::Clang;
use clang_rs_binding::index::{visit_children, ChildVisitResult, Cursor, Payload};
use clang_rs_binding::with_chdir;

fn visitor(cursor: &Cursor, _parent: &Cursor, _payload: Payload) -> i32 {
    if cursor.is_from_main_file() {
        return ChildVisitResult::CONTINUE;
    }
    let spelling = cursor.spelling();
    if cursor.is_cxx_method() || cursor.is_function_decl() || cursor.is_function_template() {
        let extent = cursor.extent();
        let start_loc = extent.start();
        let end_loc = extent.end();

        let start_spelling_loc = start_loc.spelling_location();
        let end_spelling_loc = end_loc.spelling_location();
        println!(
            "{}: {:?} - {:?}",
            spelling, start_spelling_loc, end_spelling_loc
        );
    }
    ChildVisitResult::RECURSIVE
}

#[test]
fn compile_db_works() {
    let compile_db_dir = std::path::Path::new("tests/artifacts/compiledb");
    with_chdir(compile_db_dir, || {
        std::process::Command::new("cmake")
            .arg("-Bbuild")
            .arg("-S.")
            .output()
            .unwrap_or_else(|e| panic!("failed to create compile database, {}", e));
    });

    let clang = Clang::new();
    let compiledb = clang
        .compilation_database_from_directory(compile_db_dir.join("build"))
        .unwrap();
    let compile_commands = compiledb
        .get_compile_commands(std::fs::canonicalize(compile_db_dir.join("main.cpp")).unwrap());
    assert_eq!(compile_commands.get_size(), 1);
    let compile_command = compile_commands.get_command(0);
    let index = clang.create_index_with_display_diagnostics();
    let tu = index.parse_translation_unit_from_compile_command(compile_command);
    let cursor = tu.create_cursor();
    visit_children(&cursor, visitor, std::ptr::null());
}
