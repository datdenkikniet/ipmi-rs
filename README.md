# ipmi-rs
A Rust IPMI library.

This library is based on the [IPMI v2.0 Revision 1.1][0] specification.

[0]: https://www.intel.com/content/dam/www/public/us/en/documents/product-briefs/ipmi-second-gen-interface-spec-v2-rev1-1.pdf

## Examples

This repository contains several (useful) examples, which can be found in [ipmi-rs/examples](./ipmi-rs/examples/).

They have a configurable target, using the `-c` option. It supports the formats `file://<path-to-ipmi-file>` and `rmcp://<user>:<password>@<host>:<port>`.

To see information produced by the examples, configure the log level using [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging). `info` is recommended.

### `get-info`
This example usually has to be run as root.

This example will:
1. Get SEL info
2. (If supported) get SEL allocation information
3. (If present) get the first SEL record
4. Get the Device ID
5. Get SDR info
6. Get SDR repository info
7. (If supported) get SDR allocation information
8. Load all of the SDRs from the repository
9. Attempt to read the value of all of the sensors from the SDR repository

### `sel`

This example will read out and print the SEL (System Event Log) of your target. It can also be cleared by passing the `--clear` flag.

### `ipmi-channels`
This example discovers available channels and prints channel information. For LAN channels, it also shows a small set of LAN configuration parameters (addressing and gateways).

### `ipmi-lan-config`
This example reads LAN configuration for all LAN channels and emits JSON. You can apply a JSON configuration with `--set`, print the input schema with `--print-schema`, or emit an IPv6 example payload with `--print-v6-example`.

# Project structure

This project contains three crates:

* `ipmi-rs-core`: core primitives, commands, and other application independent structures.
* `ipmi-rs`: implements IO for interacting with IPMI systems based on primitives from `ipmi-rs-core`.
* `ipmi-rs-log`: logging/formatting for items from `ipmi-rs-core` (deprecated).

# Supported commands

The following IPMI commands are currently supported in `ipmi-rs-core`:

| Command                                 | Specification section |
| :-------------------------------------- | :-------------------- |
| Get Device ID                           | 20.1                  |
| Get Channel Authentication Capabilities | 22.13                 |
| Get Channel Cipher Suites               | 22.15                 |
| Get Session Challenge                   | 22.16                 |
| Activate Session                        | 22.17                 |
| Get Channel Access                      | 22.23                 |
| Get Channel Info                        | 22.24                 |
| Set LAN Configuration Parameters        | 23.1                  |
| Get LAN Configuration Parameters        | 23.2                  |
| Get SEL Info                            | 31.2                  |
| Get SEL Allocation Info                 | 31.3                  |
| Reserve SEL                             | 31.4                  |
| Get SEL Entry                           | 31.5                  |
| Clear SEL                               | 31.9                  |
| Get Sensor Reading                      | 35.14                 |
| Get Device SDR Info                     | 35.2                  |
| Get Device SDR                          | 35.3                  |
| Get SDR Repository Info                 | 33.9                  |
| Get SDR Repository Allocation Info      | 33.10                 |
| Get SDR                                 | 33.12                 |

# Supported interfaces

## `ioctl`-based IPMI device file

The [Linux IPMI Driver][lipmid] is supported through the character device exposed by that driver, usually at `/dev/ipmi<N>`.

Access to this file generally requires root privileges.

[lipmid]: https://docs.kernel.org/driver-api/ipmi.html

## RMCP

RMCP with the following authentication types is supported:
* Unauthenticated
* MD5
* MD2

## RMCP+

RCMP+ is supported, but the subset of authentication, confidentiality, and integrity algorithms is limited.

| Authentication algorithm | Supported |
| :----------------------- | :-------- |
| RAKP-HMAC-SHA1           | Yes       |
| RAKP-None                | No        |
| RAKP-HMAC-MD5            | No        |
| RAKP-HMAC-SHA256         | No        |

| Confidentiality algorithm | Supported |
| :------------------------ | :-------- |
| None                      | Yes       |
| AES-CBC-128               | Yes       |
| xRC4-128                  | No        |
| xRC4-40                   | No        |

| Integrity algorithm | Supported |
| :------------------ | :-------- |
| None                | Yes       |
| HMAC-SHA1-96        | Yes       |
| HMAC-MD5-128        | No        |
| MD5-128             | No        |
| HMAC-SHA256-128     | No        |

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