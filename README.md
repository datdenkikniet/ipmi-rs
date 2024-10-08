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
- [ ] FRU information lookup
- [x] `ioctl`-based IPMI device file interface support
    - Supports bridging IPMI requests to other IPMBs.
- [x] RMCP
    Supported auth types:
    - [x] Unauthenticated
    - [x] MD5
    - [x] MD2
    - [ ] Key
- [x] RMCP+
    - The security aspects of the RMCP+ implementation itself is not specifically security-vetted. It _should_ be secure, but you must not rely on it.
    -  Authentication algorithms:
        -  [ ] RAKP-None
        -  [x] RAKP-HMAC-SHA1
        -  [ ] RAKP-HMAC-MD5
        -  [ ] RAKP-HMAC-SHA256
    - Confidentiality algorithms:
        - [x] None
        - [x] AES-CBC-128
        - [ ] xRC4-128
        - [ ] xRC4-40
    - Integrity algorithms:
        - [x] None
        - [x] HMAC-SHA1-96
        - [ ] HMAC-MD5-128
        - [ ] MD5-128
        - [ ] HMAC-SHA256-128

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

This project aims to conform to [Conventional Commits]. If you make contributions,
please be so kind to stick to that format :)

[Conventional Commits]: https://www.conventionalcommits.org/en/v1.0.0/#summary