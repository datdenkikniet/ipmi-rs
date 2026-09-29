#![cfg_attr(not(any(feature = "std", test)), no_std)]
//! ipmi-rs-core: a pure-rust, sans-IO IPMI library.
//!
//! This library provides data structures for the requests and responses
//! defined in the IPMI spec, and primitives for interacting with an IPMI connection.
//!
//! Disable default features for a `no_std`, allocation-free build. Commands can
//! encode into caller-provided buffers through [`connection::EncodeIpmiCommand`].

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod app;

pub mod connection;

pub mod storage;

pub mod sensor_event;

pub mod transport;

#[cfg(all(test, feature = "alloc"))]
mod tests;
