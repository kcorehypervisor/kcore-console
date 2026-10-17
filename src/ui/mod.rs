//! Ratatui screens for the appliance console.

mod cols;
mod diagnostics;
mod footer;
mod frame;
mod gpu;
mod header;
mod help;
mod logo;
mod network;
mod overview;
mod security;
mod storage;
mod widgets;

#[cfg(test)]
mod gallery;
#[cfg(test)]
mod visual;

pub use frame::draw;

#[cfg(test)]
pub use visual::{buffer_contains_on_one_line, buffer_plain, buffer_svg};
