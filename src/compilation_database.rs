use std::marker::PhantomData;
use std::path::Path;

use crate::clang::Clang;
use crate::utility::{cxstring_into_string, path_to_cstring};

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

impl Clang {
    pub fn compilation_database_from_directory<P: AsRef<Path>>(
        &self,
        build_dir: P,
    ) -> Result<CompilationDatabase<'_>, CompilationDatabaseError> {
        let mut error = clang_sys::CXCompilationDatabase_NoError;
        let raw = unsafe {
            clang_sys::clang_CompilationDatabase_fromDirectory(
                path_to_cstring(build_dir).as_ptr(),
                &mut error,
            )
        };
        match error {
            clang_sys::CXCompilationDatabase_NoError => Ok(CompilationDatabase {
                raw,
                _clang: PhantomData,
            }),
            clang_sys::CXCompilationDatabase_CanNotLoadDatabase => {
                Err(CompilationDatabaseError::CanNotLoadDatabase)
            }
            e => unreachable!("unexpected CXCompilationDatabase error {}", e),
        }
    }
}

#[derive(Debug)]
pub struct CompileCommands<'compile_db> {
    raw: clang_sys::CXCompileCommands,
    _compile_db: PhantomData<&'compile_db CompilationDatabase<'compile_db>>,
}

impl<'compile_db> Drop for CompileCommands<'compile_db> {
    fn drop(&mut self) {
        unsafe {
            clang_sys::clang_CompileCommands_dispose(self.raw);
        }
    }
}

impl<'clang> CompilationDatabase<'clang> {
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
        assert!(!raw.is_null());
        CompileCommands {
            raw,
            _compile_db: PhantomData,
        }
    }
}

impl<'compile_commands> CompileCommands<'compile_commands> {
    pub fn get_size(&self) -> u32 {
        unsafe { clang_sys::clang_CompileCommands_getSize(self.raw) }
    }
}

#[derive(Debug)]
pub struct CompileCommand<'compile_commands> {
    pub(crate) raw: clang_sys::CXCompileCommand,
    _compile_commands: PhantomData<&'compile_commands CompileCommands<'compile_commands>>,
}

impl<'compile_commands> CompileCommands<'compile_commands> {
    pub fn get_command(&self, index: u32) -> CompileCommand<'_> {
        let raw = unsafe { clang_sys::clang_CompileCommands_getCommand(self.raw, index) };
        assert!(!raw.is_null());
        CompileCommand {
            raw,
            _compile_commands: PhantomData,
        }
    }
}

impl<'compile_command> CompileCommand<'compile_command> {
    pub fn get_num_args(&self) -> u32 {
        unsafe { clang_sys::clang_CompileCommand_getNumArgs(self.raw) }
    }
    pub fn get_arg(&self, index: u32) -> String {
        unsafe { cxstring_into_string(clang_sys::clang_CompileCommand_getArg(self.raw, index)) }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        use crate::utility::traits::*;

        let _clang = Clang::new();

        let error = CompilationDatabaseError::CanNotLoadDatabase;
        is_small_value_enum(&error);

        let compile_db = CompilationDatabase {
            raw: std::ptr::null_mut() as clang_sys::CXCompilationDatabase,
            _clang: PhantomData,
        };
        is_ffi_struct(&compile_db);

        let compile_commands = CompileCommands {
            raw: std::ptr::null_mut() as clang_sys::CXCompileCommands,
            _compile_db: PhantomData,
        };
        is_ffi_struct(&compile_commands);

        let compile_command = CompileCommand {
            raw: std::ptr::null_mut() as clang_sys::CXCompileCommand,
            _compile_commands: PhantomData,
        };
        is_ffi_struct(&compile_command);
    }
}
