# ipmi-rs
A Rust IPMI library.

## Examples
### `get-info`
Configure the log level of this example with `RUST_LOG`. `info` is recommended.

This example usually has to be run as root.

This example will, using the `/dev/ipmi0` file:
1. Get SEL info
2. (If supported) get SEL allocation information
3. (If present) get the first SEL record
4. Get the Device ID
5. Get SDR info
6. Get SDR repository info
7. (If supported) get SDR allocation information
8. Load all of the SDRs from the repository
9. Attempt to read the value of all of the sensors from the SDR repository

# Features
- [x] SEL info
- [x] SDR repository info
- [x] Get SDR repository entries
- [x] Read sensor data from sensors obtained from SDR repository
- [x] `ioctl`-based IPMI device file interface support
- [ ] Other IPMI interfaces
- [ ] More?

## License

All source code (including code snippets) is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  [https://www.apache.org/licenses/LICENSE-2.0][L1])
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  [https://opensource.org/licenses/MIT][L2])

[L1]: https://www.apache.org/licenses/LICENSE-2.0
[L2]: https://opensource.org/licenses/MIT

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.