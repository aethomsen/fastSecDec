//! Arithmetic-only native algebraic callbacks. Generation supplies a checked
//! branch contract; saved execution restores this association, not its proof.
mod numeric;
mod program;
mod shifted;
#[cfg(feature = "threshold-decomposition")]
pub use numeric::Number;
use program::resolve;
pub use program::{RootProgram, Scope};
use std::cell::RefCell;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported algebraic callback: {0}")]
    Unsupported(String),
    #[error("invalid algebraic callback: {0}")]
    Invalid(String),
    #[cfg(feature = "threshold-decomposition")]
    #[error("native algebraic callback: {0}")]
    Native(String),
}
thread_local! { static FAILURE: RefCell<Option<String>> = const { RefCell::new(None) }; }
pub fn isolated_attempt<R>(run: impl FnOnce() -> R) -> (R, Option<String>) {
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) {
            FAILURE.replace(self.0.take());
        }
    }
    let _restore = Restore(FAILURE.take());
    let value = run();
    (value, FAILURE.take())
}
#[cfg(feature = "threshold-decomposition")]
pub fn attempt<R>(run: impl FnOnce() -> R) -> Result<R, String> {
    let (value, failure) = isolated_attempt(run);
    match failure {
        Some(error) => Err(error),
        None => Ok(value),
    }
}
fn failed(error: String) {
    FAILURE.with_borrow_mut(|slot| {
        if slot.is_none() {
            *slot = Some(error);
        }
    });
}
pub(crate) fn register() {
    let _ = *numeric::ROOT;
    let _ = *numeric::ROOT_V2;
    let _ = *numeric::ROOT_V3;
}
pub(crate) fn is_root(symbol: symbolica::atom::Symbol) -> bool {
    symbol == *numeric::ROOT || symbol == *numeric::ROOT_V2 || symbol == *numeric::ROOT_V3
}

#[cfg(all(test, feature = "threshold-decomposition"))]
mod tests;

#[cfg(all(test, feature = "threshold-decomposition"))]
mod affine_tests;

#[cfg(all(test, feature = "threshold-decomposition"))]
mod shifted_tests;
