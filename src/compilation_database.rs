use std::marker::PhantomData;
use std::path::Path;

use crate::clang::Clang;
use crate::utility::path_to_cstring;

#[derive(Debug)]
pub struct CompilationDatabase<'clang> {
    raw: clang_sys::CXCompilationDatabase,
    _clang: PhantomData<&'clang Clang>,
}

impl<'clang> Drop for CompilationDatabase<'clang> {
    fn drop(&mut self) {
        unsafe {
            clang_sys::clang_CompilationDatabase_dispose(self.raw);
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompilationDatabaseError {
    CanNotLoadDatabase,
}

#[derive(Debug)]
pub struct CompileCommands<'compile_db> {
    raw: clang_sys::CXCompileCommands,
    _compile_db: PhantomData<&'compile_db CompilationDatabase<'compile_db>>,
}

impl<'compile_db> Drop for CompileCommands<'compile_db> {
    fn drop(&mut self) {
        unsafe { clang_sys::clang_CompileCommands_dispose(self.raw) }
    }
}

impl<'clang> CompilationDatabase<'clang> {
    pub fn from_directory<P: AsRef<Path>>(build_dir: P) -> Result<Self, CompilationDatabaseError> {
        let mut error = clang_sys::CXCompilationDatabase_NoError;
        let raw = unsafe {
            clang_sys::clang_CompilationDatabase_fromDirectory(
                path_to_cstring(build_dir).as_ptr(),
                &mut error,
            )
        };
        match error {
            clang_sys::CXCompilationDatabase_NoError => Ok(Self {
                raw,
                _clang: PhantomData,
            }),
            clang_sys::CXCompilationDatabase_CanNotLoadDatabase => {
                Err(CompilationDatabaseError::CanNotLoadDatabase)
            }
            e => unreachable!("unexpected CXCompilationDatabase error {}", e),
        }
    }

    pub fn get_compile_commands<P: AsRef<Path>>(
        &self,
        complete_filename: P,
    ) -> CompileCommands<'_> {
        let raw = unsafe {
            clang_sys::clang_CompilationDatabase_getCompileCommands(
                self.raw,
                path_to_cstring(complete_filename).as_ptr(),
            )
        };
        CompileCommands {
            raw,
            _compile_db: PhantomData,
        }
    }
}
