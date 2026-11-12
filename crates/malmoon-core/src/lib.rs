//! Logic shared by every Malmoon client and the gateway.
//!
//! This crate is pure: no I/O, threads, clocks, or audio devices. That keeps it usable from the
//! desktop app, the gateway, WebAssembly, and mobile bindings alike.

#![forbid(unsafe_code)]

pub mod caption;
pub mod linebreak;
pub mod protocol;
