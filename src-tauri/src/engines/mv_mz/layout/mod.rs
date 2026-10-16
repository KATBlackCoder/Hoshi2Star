//! Text fitting for RPG Maker MZ: lays translated dialogue and descriptions
//! out the way the engine draws them (game font metrics, portraits, box
//! height, page breaks), so long translations never run off the box.
//!
//! Applied at export only: translations in the database stay line-for-line.

pub mod fit;
pub mod font;
pub mod geometry;
pub mod message;
pub mod plan;
pub mod simulate;
pub mod text;
pub mod verify;
pub mod wrap;
