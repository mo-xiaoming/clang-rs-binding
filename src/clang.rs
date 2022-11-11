use std::marker::PhantomData;

thread_local! {
    // no synchronization needed, since `Clang` is not sync or send
    static CLANG_INIT_FLAG: std::cell::Cell<i32> = std::cell::Cell::new(0);
}

/// `Clang` can only be created once per thread, and it is not `Sync` or `Send`.
///
/// ```compile_fail
/// fn sync_send<T: Sync + Send>(_: &T) {}
///
/// let clang = clang_transformer::clang::Clang::new();
/// sync_send(&clang);
/// ```
#[derive(Debug)]
pub struct Clang(PhantomData<*const ()>);

impl Clang {
    pub fn new() -> Self {
        CLANG_INIT_FLAG.with(|f| {
            if f.get() == 0 {
                clang_sys::load().unwrap();
            }
            f.set(f.get() + 1);
        });

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
        CLANG_INIT_FLAG.with(|f| {
            f.set(f.get() - 1);
            if f.get() == 0 {
                clang_sys::unload().unwrap();
            }
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn init_clang_multiple_times_is_allowed() {
        let clang_ref = || CLANG_INIT_FLAG.with(|f| f.get());

        let start_ref = clang_ref();
        assert_eq!(start_ref, 0); // this makes this test has to be the first executed?
        {
            let _clang1 = Clang::new();
            assert_eq!(clang_ref(), start_ref + 1);
            let _clang2 = Clang::new();
            assert_eq!(clang_ref(), start_ref + 2);
        }
        assert_eq!(clang_ref(), start_ref);
    }

    #[test]
    fn traits() {
        use crate::utility::traits::*;
        is_ffi_struct(&Clang::new());
    }
}
