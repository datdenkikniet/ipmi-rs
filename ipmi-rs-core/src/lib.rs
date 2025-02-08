//! IPMI-rs: a pure-rust IPMI library.
//!
//! This library provides command serialization and deserialization (in the [`app`], [`storage`] and [`sensor_event`] modules),
//! and different ways of connecting to an IPMI device (in the [`connection`] module).

pub mod app;

pub mod connection;

pub mod storage;
pub use storage::sdr::record::WithSensorRecordCommon;

pub mod sensor_event;

#[cfg(test)]
mod tests;
