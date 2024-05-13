# Unreleased
* Support for sending bridged IPMB messages for sensors that are not available on the system
  interface for File-based connections. `GetSensorReading::new()` and `GetSensorReading::for_sensor()`
  have been replaced with`GetSensorReading::for_sensor_key()` which now takes a `&Sensorkey`. ([#6])
* Fix parsing ID String modifier in `CompactSensorRecord` ([#7])
* Validate sequence numbers for `File` connections. ([#11])
* Breaking change: rename `Loggable::into_log` to `Loggable::as_log` as part of clippy cleanup. ([#12])
* Breaking change: use newtype & typesafe variants for `Channel` and `ChannelNumber` in relevant places. ([#14])
* Breaking change: remove duplicate `get_channel_authentication_capabilities` file. ([#14])
* Rudimentary RMCP+ support. ([#13])
* Add more SDR record types. ([#10], [#18])
* Breaking change: improve error reporting in SDR records. ([#10], [#18])

[#6]: https://github.com/datdenkikniet/ipmi-rs/pull/6
[#7]: https://github.com/datdenkikniet/ipmi-rs/pull/7
[#10]: https://github.com/datdenkikniet/ipmi-rs/pull/10
[#11]: https://github.com/datdenkikniet/ipmi-rs/pull/11
[#12]: https://github.com/datdenkikniet/ipmi-rs/pull/12
[#13]: https://github.com/datdenkikniet/ipmi-rs/pull/13
[#14]: https://github.com/datdenkikniet/ipmi-rs/pull/14
[#18]: https://github.com/datdenkikniet/ipmi-rs/pull/18

# [0.2.1](https://github.com/datdenkikniet/ipmi-rs/tree/v0.2.1)

* Use correct bit for detecting signedness of full-record sensor scaling field `B`. ([#4])

[#4]: https://github.com/datdenkikniet/ipmi-rs/pull/4

# [0.2.0](https://github.com/datdenkikniet/ipmi-rs/tree/v0.2.0)

Initial release.