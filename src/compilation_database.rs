use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::path::Path;

use crate::clang::Clang;
use crate::utility::path_to_cstring;

#[derive(Debug)]
pub struct CompilationDatabase<'clang> {
    raw: clang_sys::CXCompilationDatabase,
    _clang: PhantomData<&'clang Clang>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompilationDatabaseError {
    CanNotLoadDatabase,
}

impl<'clang> CompilationDatabase<'clang> {
    pub fn from_directory<P: AsRef<Path>>(build_dir: P) -> Result<Self, CompilationDatabaseError> {
        unsafe {
            let mut error = MaybeUninit::uninit();
            let raw = clang_sys::clang_CompilationDatabase_fromDirectory(
                path_to_cstring(build_dir).as_ptr(),
                error.as_mut_ptr(),
            );
            match error.assume_init() {
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
    }
}

impl<'clang> Drop for CompilationDatabase<'clang> {
    fn drop(&mut self) {
        unsafe {
            clang_sys::clang_CompilationDatabase_dispose(self.raw);
        }
    }
}
