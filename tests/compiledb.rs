use clang_rs_binding::clang::Clang;
use clang_rs_binding::index::{from_payload, to_payload, ChildVisitResult, Cursor, Payload};
use clang_rs_binding::with_chdir;
use std::path::Path;

fn visitor(cursor: &Cursor, _parent: &Cursor, payload: Payload) -> i32 {
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
        let payload = unsafe { from_payload::<AstDataPayload>(payload) };
        let new_buf = payload.borrow().clone()
            + &format!(
                "{}: {:?} - {:?}\n",
                spelling, start_spelling_loc, end_spelling_loc
            );
        *payload.borrow_mut() = new_buf;
    }
    ChildVisitResult::RECURSIVE
}

fn get_system_headers() -> Vec<String> {
    let output = std::process::Command::new("clang++")
        .arg("-v")
        .arg("-c")
        .arg("-fsyntax-only")
        .arg("main.cpp")
        .output()
        .unwrap_or_else(|e| panic!("failed to get system headers, {}", e));
    std::str::from_utf8(&output.stderr)
        .unwrap()
        .lines()
        .skip_while(|&e| e != "#include <...> search starts here:")
        .skip(1)
        .take_while(|&e| e != "End of search list.")
        .map(|e| e.trim_start().to_owned())
        .collect::<Vec<_>>()
}

type AstDataPayload = std::cell::RefCell<String>;

fn collect_ast(cursor: &Cursor<'_>) -> String {
    let data = AstDataPayload::default();
    cursor.visit_children(visitor, to_payload(&data));
    data.take()
}

fn read_test_oracle<P: AsRef<Path>>(filename: P) -> String {
    std::fs::read_to_string(filename).unwrap()
}

#[test]
fn compile_db_works() {
    let compile_db_dir = std::path::Path::new("tests/artifacts/compiledb");
    let oracle = read_test_oracle(compile_db_dir.join("compiledb.test_oracle"));
    with_chdir(compile_db_dir, || {
        std::process::Command::new("cmake")
            .arg("-Bbuild")
            .arg("-S.")
            .output()
            .unwrap_or_else(|e| panic!("failed to create compile database, {}", e));
        // must add `clang++ -v -c main.cpp`, otherwise system headers will not be found
        let system_headers = get_system_headers().join(":");
        assert!(!system_headers.is_empty());
        std::env::set_var("CPLUS_INCLUDE_PATH", system_headers);
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
    let buf = collect_ast(&cursor);

    assert_eq!(buf, oracle);
}
