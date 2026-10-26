//! What League's own classes mean, read out of an open bin document.
//!
//! `ltk-manager-bin` owns the document and the names, and `ltk-manager-assets` where an
//! asset lives. This crate sits above them and owns the classes, so neither learns what a
//! `MapContainer` is.

pub mod champions;
pub mod character;
pub mod lookup;
pub mod map;
pub mod material;
pub mod program;
mod resolver;
pub mod skin;
pub mod spell;
pub mod vfx;
