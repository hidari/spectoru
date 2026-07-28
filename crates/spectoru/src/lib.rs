//! Spectoru — Visualize tests as living specifications.
//!
//! Spectoru parses Rust and Vitest test sources to produce a static site that
//! exposes test names as the project's executable specification. The crate is
//! organized around a hexagonal core: pure domain logic lives under [`core`],
//! external library boundaries are expressed as traits in [`ports`], and
//! library-specific implementations live under [`adapters`].

pub mod adapters;
pub mod app;
pub mod core;
pub mod error;
pub mod ports;
