//! Core engine of bada ime. Behaviour is defined in `docs/spec.md` and `docs/layouts/13-bada.md`.
//!
//! The engine is event in → output out, with no I/O: the macOS shell loads the data files, passes
//! key events and host text in, and applies the [`Output`] to the text field.

pub mod data;
pub mod engine;
pub mod ffi;
pub mod hangul;
pub mod hanja;
pub mod hold;
pub mod japanese;
pub mod kanji;
pub mod kana_korean;
pub mod korean_roman;
pub mod bada_roman;
pub mod key;
pub mod keymodel;
pub mod layout;
pub mod particles;
pub mod romaji;
pub mod search;
pub mod typo;
pub mod userdict;
pub mod width;

pub use data::Data;
pub use engine::{Engine, Output, Settings};
pub use key::{Key, Mods};
