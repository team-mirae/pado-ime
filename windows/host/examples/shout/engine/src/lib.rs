//! shout's C ABI (`shout_new`, `shout_key`, …, `shout_state`), exported from this cdylib: the
//! symbols are bada-compatible's `export!` in the dependency, which a cdylib exports as its own.
pub use shout::*;
