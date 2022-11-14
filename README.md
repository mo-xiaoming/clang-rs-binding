# Baby Steps With LibClang ("idiomatic" rust version)

[![CI](https://github.com/mo-xiaoming/clang-rs-binding/actions/workflows/build.yml/badge.svg)](https://github.com/mo-xiaoming/clang-rs-binding/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/mo-xiaoming/clang-rs-binding/branch/main/graph/badge.svg?token=6WMDKF1RCK)](https://codecov.io/gh/mo-xiaoming/clang-rs-binding)

Inspired by [Bastian Rieck](https://bastian.rieck.me)'s wonderful blogs, [Baby Steps With LibClang: Walking an Abstract Syntax Tree](https://bastian.rieck.me/blog/posts/2015/baby_steps_libclang_ast/) and [Baby Steps With LibClang: Counting Function Extents](https://bastian.rieck.me/blog/posts/2016/baby_steps_libclang_function_extents/)

Two blog's corresponding rust code

- [Walking an Abstract Syntax Tree](tests/traverse_ast.rs)
- [Counting Function Extends](tests/compiledb.rs)

## A Simple Demo

For a C++ file `traverse_ast.cpp` like

```cpp
template <typename T>
bool f(T x) {
  return x % 2;
}
```

To get output like

```text
FunctionTemplate (f)
TemplateTypeParameter (T)
ParmDecl (x)
TypeRef (T)
CompoundStmt ()
ReturnStmt ()
BinaryOperator ()
DeclRefExpr (x)
IntegerLiteral ()
```

You can use something like

```rust
use std::path::Path;
use clang_rs_binding::index::{Cursor, Payload, ChildVisitResult};

fn visitor(cursor: &Cursor, _parent: &Cursor, payload: Payload) -> i32 {
    if cursor.is_from_main_file() {
        return ChildVisitResult::CONTINUE;
    }
    let cursor_kind_spelling = cursor.kind_spelling();
    let cursor_spelling = cursor.spelling();
    println!("{} ({})", cursor_kind_spelling, cursor_spelling);
    cursor.visit_children(visitor, payload);
    ChildVisitResult::CONTINUE
}

fn generate_ast<P: AsRef<Path>>(filename: P) -> impl AsRef<Path> {
    let ast_filename = Path::new("traverse_ast.ast");
    std::process::Command::new("clang++")
        .arg("-emit-ast").arg(filename.as_ref()).status().unwrap();
    ast_filename
}

fn main() {
    let traverse_ast_dir = Path::new("tests/artifacts/traverse_ast");
    let ast_filename = generate_ast(traverse_ast_dir.join("traverse_ast.cpp"));

    clang_rs_binding::clang::Clang::default()
        .create_index_with_display_diagnostics()
        .create_translation_unit(&ast_filename)
        .create_cursor()
        .visit_children(visitor, std::ptr::null());
}
```
