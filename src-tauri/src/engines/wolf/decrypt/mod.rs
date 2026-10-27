//! Wolf RPG `.wolf` archive decryption — one submodule per encryption scheme.
//!
//! - [`legacy_xor`] — XOR-12 DXA archives (Wolf v1.x, v2.x, v3.0–3.31). Native,
//!   fully implemented (F4-02).
//! - [`wolfx`] — ChaCha20-based WolfX encryption (Wolf v3.5+ Pro). Not
//!   implemented natively; users decrypt with UberWolf first (see module docs).
//! - [`writer`] — rebuilds a v6/v8 archive with translated files (plan RW, RW-3).
//!
//! Downstream code (`extractor`, `detector`) dispatches on archive bytes, never
//! on a user-selected mode.

pub mod legacy_xor;
pub mod wolfx;
pub mod writer;
