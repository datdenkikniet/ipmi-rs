# Unreleased
* Support for sending bridged IPMB messages for sensors that are not available on the system interface.
  `GetSensorReading::new()` and `GetSensorReading::for_sensor()` have been replaced with
  `GetSensorReading::for_sensor_key()` which now takes a `&Sensorkey`. ([#6])
  * Currently, this is only implemented for file-based connections - RMCP is not supported yet.
  * We determine whether a sensor is available on the system interface by checking
    the value for the sensor owner ID and comparing it to the value returned by the
    `ipmi_get_my_address` ioctl. If the ioctl fails, we assume the default address of 0x20.
    This may break on systems that use a non-standard default address and do not return that
    address.

[#6]: https://github.com/datdenkikniet/ipmi-rs/pull/6

# [0.2.1](https://github.com/datdenkikniet/ipmi-rs/tree/v0.2.1)

* Use correct bit for detecting signedness of full-record sensor scaling field `B`. ([#4])

[#4]: https://github.com/datdenkikniet/ipmi-rs/pull/4

# [0.2.0](https://github.com/datdenkikniet/ipmi-rs/tree/v0.2.0)

Initial release.