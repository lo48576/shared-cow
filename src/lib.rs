//! Shared clone-on-write type.
#![feature(rust_2018_preview)]
#![feature(macro_vis_matcher)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub use crate::arc_cow::ArcCow;

mod arc_cow;
