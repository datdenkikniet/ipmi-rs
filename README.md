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