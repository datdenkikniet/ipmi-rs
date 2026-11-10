# `ipmi-rs-core`: IPMI specification definitions

This crate contains the definitions for IPMI commands, payloads, and other data structures used in the
IPMI protocol.

The goal of this library is to be a sans-IO wrapper that can be re-used by other implementations.

For higher-level details, such as a file-based or RMCP connection, check out the [`ipmi-rs`] crate, which is built
on top of `ipmi-rs-core`.

## `no_std`

`ipmi-rs-core` supports targets without `std` or a global allocator. Disable the default features for the allocation-free API:

```toml
ipmi-rs-core = { version = "0.6.0", default-features = false }
```

Commands implement `EncodeIpmiCommand`, which writes into caller-provided storage:

```rust
use ipmi_rs_core::{
    app::GetChannelInfo,
    connection::{Channel, EncodeIpmiCommand},
};

let command = GetChannelInfo::new(Channel::Current);
let mut data = [0_u8; 1];
let request = command.encode_request(&mut data)?;
```

Response parsers consume borrowed byte slices. Parsers whose output types do not own a `Vec` or `String` are also allocation-free. Enable the `alloc` feature for the existing owned message and variable-length response APIs. The optional `log` feature is compatible with `no_std`.

[`ipmi-rs`]: https://crates.io/crates/ipmi-rs