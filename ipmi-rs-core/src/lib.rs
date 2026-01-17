//! ipmi-rs-core: a pure-rust, sans-IO IPMI library.
//!
//! This library provides data structures for the requests and responses
//! defined in the IPMI spec, and primitives for interacting with an IPMI connection.

pub mod app;

pub mod connection;

pub mod storage;

pub mod sensor_event;

pub mod transport;

#[cfg(test)]
mod tests;
