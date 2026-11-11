//! bada's C ABI (`bada_data_new`, `bada_new`, `bada_key`, …, `bada_state`), exported from this
//! cdylib: the `#[no_mangle]` functions of `bada::ffi`, which a cdylib exports as its own.
pub use bada::ffi::*;
