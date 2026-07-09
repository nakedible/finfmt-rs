//! Assembly inspection wrappers (`asm-inspect` feature only).
//!
//! Each public primitive is wrapped here with `#[inline(never)]` and the
//! parameters typically baked in by the const generics of a Field
//! composition. `cargo asm --features asm-inspect <path>` then shows what the
//! optimizer emits for that specialization.
//!
//! Wrappers are organized by primitive module. Functions whose codegen
//! materially differs across parameter values get multiple variants
//! (e.g. left- vs right-aligned `pack_nibbles`).

pub mod bertlv;
pub mod bitmap;
pub mod bytes;
pub mod decimal;
pub mod ebcdic;
pub mod int;
pub mod nibble;
pub mod text;
pub mod validation;
