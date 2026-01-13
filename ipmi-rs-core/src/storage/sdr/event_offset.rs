//! Sensor-Specific Event Offset Decoding
//!
//! Reference: IPMI 2.0 Specification, Table 42-3 "Sensor Type Codes and Data"
//!
//! This module provides human-readable descriptions for sensor-specific event offsets.
//! Event offsets are 4-bit values (0-15) that describe the specific event that occurred
//! for a given sensor type.

use core::fmt;

use super::SensorType;

fn sensor_event_offset_desc(sensor_type: SensorType, offset: u8) -> Option<&'static str> {
    let offset = offset & 0x0F; // Only lower 4 bits are used

    match sensor_type {
        // Physical Security (Chassis Intrusion) - 05h
        SensorType::ChassisIntrusion => match offset {
            0x00 => Some("General Chassis Intrusion"),
            0x01 => Some("Drive Bay intrusion"),
            0x02 => Some("I/O Card area intrusion"),
            0x03 => Some("Processor area intrusion"),
            0x04 => Some("LAN Leash Lost"),
            0x05 => Some("Unauthorized dock/undock"),
            0x06 => Some("FAN area intrusion"),
            _ => None,
        },

        // Platform Security Violation Attempt - 06h
        SensorType::PlatformSecurityViolationAttempt => match offset {
            0x00 => Some("Secure Mode Violation Attempt"),
            0x01 => Some("Pre-boot Password Violation - user password"),
            0x02 => Some("Pre-boot Password Violation - setup password"),
            0x03 => Some("Pre-boot Password Violation - network boot password"),
            0x04 => Some("Other pre-boot Password Violation"),
            0x05 => Some("Out-of-band Access Password Violation"),
            _ => None,
        },

        // Processor - 07h
        SensorType::Processor => match offset {
            0x00 => Some("IERR"),
            0x01 => Some("Thermal Trip"),
            0x02 => Some("FRB1/BIST failure"),
            0x03 => Some("FRB2/Hang in POST failure"),
            0x04 => Some("FRB3/Processor Startup/Initialization failure"),
            0x05 => Some("Configuration Error"),
            0x06 => Some("SM BIOS 'Uncorrectable CPU-complex Error'"),
            0x07 => Some("Processor Presence detected"),
            0x08 => Some("Processor disabled"),
            0x09 => Some("Terminator Presence Detected"),
            0x0A => Some("Processor Automatically Throttled"),
            0x0B => Some("Machine Check Exception (Uncorrectable)"),
            0x0C => Some("Correctable Machine Check Error"),
            _ => None,
        },

        // Power Supply - 08h
        SensorType::PowerSupply => match offset {
            0x00 => Some("Presence detected"),
            0x01 => Some("Power Supply Failure detected"),
            0x02 => Some("Predictive Failure"),
            0x03 => Some("Power Supply input lost (AC/DC)"),
            0x04 => Some("Power Supply input lost or out-of-range"),
            0x05 => Some("Power Supply input out-of-range, but present"),
            0x06 => Some("Configuration error"),
            0x07 => Some("Power Supply Inactive"),
            _ => None,
        },

        // Power Unit - 09h
        SensorType::PowerUnit => match offset {
            0x00 => Some("Power Off / Power Down"),
            0x01 => Some("Power Cycle"),
            0x02 => Some("240VA Power Down"),
            0x03 => Some("Interlock Power Down"),
            0x04 => Some("AC lost / Power input lost"),
            0x05 => Some("Soft Power Control Failure"),
            0x06 => Some("Power Unit Failure detected"),
            0x07 => Some("Predictive Failure"),
            _ => None,
        },

        // Memory - 0Ch
        SensorType::Memory => match offset {
            0x00 => Some("Correctable ECC / other correctable memory error"),
            0x01 => Some("Uncorrectable ECC / other uncorrectable memory error"),
            0x02 => Some("Parity"),
            0x03 => Some("Memory Scrub Failed"),
            0x04 => Some("Memory Device Disabled"),
            0x05 => Some("Correctable ECC / other correctable memory error logging limit reached"),
            0x06 => Some("Presence detected"),
            0x07 => Some("Configuration error"),
            0x08 => Some("Spare"),
            0x09 => Some("Memory Automatically Throttled"),
            0x0A => Some("Critical Overtemperature"),
            _ => None,
        },

        // Drive Slot (Bay) - 0Dh
        SensorType::DriveSlotBay => match offset {
            0x00 => Some("Drive Presence"),
            0x01 => Some("Drive Fault"),
            0x02 => Some("Predictive Failure"),
            0x03 => Some("Hot Spare"),
            0x04 => Some("Consistency Check / Parity Check in progress"),
            0x05 => Some("In Critical Array"),
            0x06 => Some("In Failed Array"),
            0x07 => Some("Rebuild/Remap in progress"),
            0x08 => Some("Rebuild/Remap Aborted"),
            _ => None,
        },

        // System Firmware Progress - 0Fh
        SensorType::SystemFirmwareProgress => match offset {
            0x00 => Some("System Firmware Error"),
            0x01 => Some("System Firmware Hang"),
            0x02 => Some("System Firmware Progress"),
            _ => None,
        },

        // Event Logging Disabled - 10h
        SensorType::EventLoggingDisabled => match offset {
            0x00 => Some("Correctable Memory Error Logging Disabled"),
            0x01 => Some("Event 'Type' Logging Disabled"),
            0x02 => Some("Log Area Reset/Cleared"),
            0x03 => Some("All Event Logging Disabled"),
            0x04 => Some("SEL Full"),
            0x05 => Some("SEL Almost Full"),
            0x06 => Some("Correctable Machine Check Error Logging Disabled"),
            _ => None,
        },

        // Watchdog 1 - 11h
        SensorType::Watchdog1 => match offset {
            0x00 => Some("BIOS Watchdog Reset"),
            0x01 => Some("OS Watchdog Reset"),
            0x02 => Some("OS Watchdog Shut Down"),
            0x03 => Some("OS Watchdog Power Down"),
            0x04 => Some("OS Watchdog Power Cycle"),
            0x05 => Some("OS Watchdog NMI / Diagnostic Interrupt"),
            0x06 => Some("OS Watchdog Expired, status only"),
            0x07 => Some("OS Watchdog pre-timeout Interrupt, non-NMI"),
            _ => None,
        },

        // System Event - 12h
        SensorType::SystemEvent => match offset {
            0x00 => Some("System Reconfigured"),
            0x01 => Some("OEM System Boot Event"),
            0x02 => Some("Undetermined system hardware failure"),
            0x03 => Some("Entry added to Auxiliary Log"),
            0x04 => Some("PEF Action"),
            0x05 => Some("Timestamp Clock Sync"),
            _ => None,
        },

        // Critical Interrupt - 13h
        SensorType::CriticalInterrupt => match offset {
            0x00 => Some("Front Panel NMI / Diagnostic Interrupt"),
            0x01 => Some("Bus Timeout"),
            0x02 => Some("I/O channel check NMI"),
            0x03 => Some("Software NMI"),
            0x04 => Some("PCI PERR"),
            0x05 => Some("PCI SERR"),
            0x06 => Some("EISA Fail Safe Timeout"),
            0x07 => Some("Bus Correctable Error"),
            0x08 => Some("Bus Uncorrectable Error"),
            0x09 => Some("Fatal NMI"),
            0x0A => Some("Bus Fatal Error"),
            0x0B => Some("Bus Degraded"),
            _ => None,
        },

        // Button / Switch - 14h
        SensorType::ButtonOrSwitch => match offset {
            0x00 => Some("Power Button pressed"),
            0x01 => Some("Sleep Button pressed"),
            0x02 => Some("Reset Button pressed"),
            0x03 => Some("FRU latch open"),
            0x04 => Some("FRU service request button"),
            _ => None,
        },

        // Chip Set - 19h
        SensorType::ChipSet => match offset {
            0x00 => Some("Soft Power Control Failure"),
            0x01 => Some("Thermal Trip"),
            _ => None,
        },

        // System Boot / Restart Initiated - 1Dh
        SensorType::SystemBootOrRestartInitiated => match offset {
            0x00 => Some("Initiated by power up"),
            0x01 => Some("Initiated by hard reset"),
            0x02 => Some("Initiated by warm reset"),
            0x03 => Some("User requested PXE boot"),
            0x04 => Some("Automatic boot to diagnostic"),
            0x05 => Some("OS / run-time software initiated hard reset"),
            0x06 => Some("OS / run-time software initiated warm reset"),
            0x07 => Some("System Restart"),
            _ => None,
        },

        // Boot Error - 1Eh
        SensorType::BootError => match offset {
            0x00 => Some("No bootable media"),
            0x01 => Some("Non-bootable diskette left in drive"),
            0x02 => Some("PXE Server not found"),
            0x03 => Some("Invalid boot sector"),
            0x04 => Some("Timeout waiting for user selection of boot source"),
            _ => None,
        },

        // Base OS Boot / Installation Status - 1Fh
        SensorType::BaseOsBootOrInstallationStatus => match offset {
            0x00 => Some("A: boot completed"),
            0x01 => Some("C: boot completed"),
            0x02 => Some("PXE boot completed"),
            0x03 => Some("Diagnostic boot completed"),
            0x04 => Some("CD-ROM boot completed"),
            0x05 => Some("ROM boot completed"),
            0x06 => Some("boot completed - boot device not specified"),
            0x07 => Some("Base OS/Hypervisor Installation started"),
            0x08 => Some("Base OS/Hypervisor Installation completed"),
            0x09 => Some("Base OS/Hypervisor Installation aborted"),
            0x0A => Some("Base OS/Hypervisor Installation failed"),
            _ => None,
        },

        // OS Stop / Shutdown - 20h
        SensorType::OsStopOrShutdown => match offset {
            0x00 => Some("Critical stop during OS load / initialization"),
            0x01 => Some("Run-time Critical Stop"),
            0x02 => Some("OS Graceful Stop"),
            0x03 => Some("OS Graceful Shutdown"),
            0x04 => Some("Soft Shutdown initiated by PEF"),
            0x05 => Some("Agent Not Responding"),
            _ => None,
        },

        // System ACPI Power State - 22h
        SensorType::SystemACPIPowerState => match offset {
            0x00 => Some("S0 / G0 'working'"),
            0x01 => Some("S1 'sleeping with system h/w & processor context maintained'"),
            0x02 => Some("S2 'sleeping, processor context lost'"),
            0x03 => Some("S3 'sleeping, processor & h/w context lost, memory retained'"),
            0x04 => Some("S4 'non-volatile sleep / suspend-to-disk'"),
            0x05 => Some("S5 / G2 'soft-off'"),
            0x06 => Some("S4 / S5 soft-off, particular S4 / S5 state cannot be determined"),
            0x07 => Some("G3 / Mechanical Off"),
            0x08 => Some("Sleeping in an S1, S2, or S3 states"),
            0x09 => Some("G1 sleeping"),
            0x0A => Some("S5 entered by override"),
            0x0B => Some("Legacy ON state"),
            0x0C => Some("Legacy OFF state"),
            0x0E => Some("Unknown"),
            _ => None,
        },

        // Watchdog 2 - 23h
        SensorType::Watchdog2 => match offset {
            0x00 => Some("Timer expired, status only"),
            0x01 => Some("Hard Reset"),
            0x02 => Some("Power Down"),
            0x03 => Some("Power Cycle"),
            0x08 => Some("Timer interrupt"),
            _ => None,
        },

        // Platform Alert - 24h
        SensorType::PlatformAlert => match offset {
            0x00 => Some("platform generated page"),
            0x01 => Some("platform generated LAN alert"),
            0x02 => Some("Platform Event Trap generated"),
            0x03 => Some("platform generated SNMP trap"),
            _ => None,
        },

        // Entity Presence - 25h
        SensorType::EntityPresence => match offset {
            0x00 => Some("Entity Present"),
            0x01 => Some("Entity Absent"),
            0x02 => Some("Entity Disabled"),
            _ => None,
        },

        // LAN - 27h
        SensorType::LAN => match offset {
            0x00 => Some("LAN Heartbeat Lost"),
            0x01 => Some("LAN Heartbeat"),
            _ => None,
        },

        // Management Subsystem Health - 28h
        SensorType::ManagementSubSysHealth => match offset {
            0x00 => Some("sensor access degraded or unavailable"),
            0x01 => Some("controller access degraded or unavailable"),
            0x02 => Some("management controller off-line"),
            0x03 => Some("management controller unavailable"),
            0x04 => Some("Sensor failure"),
            0x05 => Some("FRU failure"),
            _ => None,
        },

        // Battery - 29h
        SensorType::Battery => match offset {
            0x00 => Some("battery low"),
            0x01 => Some("battery failed"),
            0x02 => Some("battery presence detected"),
            _ => None,
        },

        // Session Audit - 2Ah
        SensorType::SessionAudit => match offset {
            0x00 => Some("Session Activated"),
            0x01 => Some("Session Deactivated"),
            0x02 => Some("Invalid Username or Password"),
            0x03 => Some("Invalid password disable"),
            _ => None,
        },

        // Version Change - 2Bh
        SensorType::VersionChange => match offset {
            0x00 => Some("Hardware change detected with associated Entity"),
            0x01 => Some("Firmware or software change detected with associated Entity"),
            0x02 => Some("Hardware incompatibility detected with associated Entity"),
            0x03 => Some("Firmware or software incompatibility detected with associated Entity"),
            0x04 => Some("Entity is of an invalid or unsupported hardware version"),
            0x05 => Some("Entity contains an invalid or unsupported firmware or software version"),
            0x06 => Some("Hardware Change detected with associated Entity was successful"),
            0x07 => Some("Software or F/W Change detected with associated Entity was successful"),
            _ => None,
        },

        // FRU State - 2Ch
        SensorType::FRUState => match offset {
            0x00 => Some("FRU Not Installed"),
            0x01 => Some("FRU Inactive"),
            0x02 => Some("FRU Activation Requested"),
            0x03 => Some("FRU Activation In Progress"),
            0x04 => Some("FRU Active"),
            0x05 => Some("FRU Deactivation Requested"),
            0x06 => Some("FRU Deactivation In Progress"),
            0x07 => Some("FRU Communication Lost"),
            _ => None,
        },

        // Other types without sensor-specific offsets
        _ => None,
    }
}

/// Decode a sensor-specific event offset to a human-readable description.
///
/// # Arguments
/// * `sensor_type` - The sensor type code (from IPMI Table 42-3)
/// * `offset` - The event offset (bits [3:0] of the event type/reading byte)
///
/// Reference: IPMI 2.0 Specification, Table 42-3
fn decode_sensor_event_offset(
    f: &mut fmt::Formatter<'_>,
    sensor_type: SensorType,
    offset: u8,
) -> fmt::Result {
    if let Some(desc) = sensor_event_offset_desc(sensor_type, offset) {
        f.write_str(desc)
    } else {
        Ok(())
    }
}

fn generic_event_offset_desc(event_type: u8, offset: u8) -> Option<&'static str> {
    let offset = offset & 0x0F;

    match event_type {
        // Threshold - 01h
        0x01 => match offset {
            0x00 => Some("Lower Non-critical - going low"),
            0x01 => Some("Lower Non-critical - going high"),
            0x02 => Some("Lower Critical - going low"),
            0x03 => Some("Lower Critical - going high"),
            0x04 => Some("Lower Non-recoverable - going low"),
            0x05 => Some("Lower Non-recoverable - going high"),
            0x06 => Some("Upper Non-critical - going low"),
            0x07 => Some("Upper Non-critical - going high"),
            0x08 => Some("Upper Critical - going low"),
            0x09 => Some("Upper Critical - going high"),
            0x0A => Some("Upper Non-recoverable - going low"),
            0x0B => Some("Upper Non-recoverable - going high"),
            _ => None,
        },

        // Usage State (Discrete) - 02h
        0x02 => match offset {
            0x00 => Some("Transition to Idle"),
            0x01 => Some("Transition to Active"),
            0x02 => Some("Transition to Busy"),
            _ => None,
        },

        // State (Discrete) - 03h
        0x03 => match offset {
            0x00 => Some("State Deasserted"),
            0x01 => Some("State Asserted"),
            _ => None,
        },

        // Predictive Failure (Discrete) - 04h
        0x04 => match offset {
            0x00 => Some("Predictive Failure deasserted"),
            0x01 => Some("Predictive Failure asserted"),
            _ => None,
        },

        // Limit (Discrete) - 05h
        0x05 => match offset {
            0x00 => Some("Limit Not Exceeded"),
            0x01 => Some("Limit Exceeded"),
            _ => None,
        },

        // Performance (Discrete) - 06h
        0x06 => match offset {
            0x00 => Some("Performance Met"),
            0x01 => Some("Performance Lags"),
            _ => None,
        },

        // Severity (Discrete) - 07h
        0x07 => match offset {
            0x00 => Some("transition to OK"),
            0x01 => Some("transition to Non-Critical from OK"),
            0x02 => Some("transition to Critical from less severe"),
            0x03 => Some("transition to Non-recoverable from less severe"),
            0x04 => Some("transition to Non-Critical from more severe"),
            0x05 => Some("transition to Critical from Non-recoverable"),
            0x06 => Some("transition to Non-recoverable"),
            0x07 => Some("Monitor"),
            0x08 => Some("Informational"),
            _ => None,
        },

        // Device Presence (Discrete) - 08h
        0x08 => match offset {
            0x00 => Some("Device Removed / Device Absent"),
            0x01 => Some("Device Inserted / Device Present"),
            _ => None,
        },

        // Device Enabled (Discrete) - 09h
        0x09 => match offset {
            0x00 => Some("Device Disabled"),
            0x01 => Some("Device Enabled"),
            _ => None,
        },

        // Availability State (Discrete) - 0Ah
        0x0A => match offset {
            0x00 => Some("transition to Running"),
            0x01 => Some("transition to In Test"),
            0x02 => Some("transition to Power Off"),
            0x03 => Some("transition to On Line"),
            0x04 => Some("transition to Off Line"),
            0x05 => Some("transition to Off Duty"),
            0x06 => Some("transition to Degraded"),
            0x07 => Some("transition to Power Save"),
            0x08 => Some("Install Error"),
            _ => None,
        },

        // Redundancy State (Discrete) - 0Bh
        0x0B => match offset {
            0x00 => Some("Fully Redundant"),
            0x01 => Some("Redundancy Lost"),
            0x02 => Some("Redundancy Degraded"),
            0x03 => Some("Non-redundant:Sufficient Resources from Redundant"),
            0x04 => Some("Non-redundant:Sufficient Resources from Insufficient Resources"),
            0x05 => Some("Non-redundant:Insufficient Resources"),
            0x06 => Some("Redundancy Degraded from Fully Redundant"),
            0x07 => Some("Redundancy Degraded from Non-redundant"),
            _ => None,
        },

        // ACPI Device Power State (Discrete) - 0Ch
        0x0C => match offset {
            0x00 => Some("D0 Power State"),
            0x01 => Some("D1 Power State"),
            0x02 => Some("D2 Power State"),
            0x03 => Some("D3 Power State"),
            _ => None,
        },

        _ => None,
    }
}

/// Decode generic event/reading type offset to a human-readable description.
///
/// This handles generic event types (Event/Reading Type Codes 02h-0Ch)
/// as defined in IPMI 2.0 Specification, Table 42-2.
///
/// # Arguments
/// * `event_type` - The event/reading type code
/// * `offset` - The event offset (bits [3:0])
///
/// Reference: IPMI 2.0 Specification, Table 42-2
fn decode_generic_event_offset(
    f: &mut fmt::Formatter<'_>,
    event_type: u8,
    offset: u8,
) -> fmt::Result {
    if let Some(desc) = generic_event_offset_desc(event_type, offset) {
        f.write_str(desc)
    } else {
        Ok(())
    }
}

/// Decode an event to a human-readable description.
///
/// This function attempts to decode both generic event types and
/// sensor-specific events.
///
/// # Arguments
/// * `event_type` - The event/reading type code (from event data byte 0, bits [6:0])
/// * `sensor_type` - The sensor type
/// * `offset` - The event offset (bits [3:0])
pub fn decode_event(
    f: &mut fmt::Formatter<'_>,
    event_type: u8,
    sensor_type: SensorType,
    offset: u8,
) -> fmt::Result {
    let event_type_code = event_type & 0x7F;

    if event_type_code == 0x6F {
        // Sensor-specific event
        decode_sensor_event_offset(f, sensor_type, offset)
    } else if event_type_code >= 0x01 && event_type_code <= 0x0C {
        // Generic event
        decode_generic_event_offset(f, event_type_code, offset)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SensorOffset {
        sensor_type: SensorType,
        offset: u8,
    }

    impl fmt::Display for SensorOffset {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            decode_sensor_event_offset(f, self.sensor_type, self.offset)
        }
    }

    struct GenericOffset {
        event_type: u8,
        offset: u8,
    }

    impl fmt::Display for GenericOffset {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            decode_generic_event_offset(f, self.event_type, self.offset)
        }
    }

    struct EventOffset {
        event_type: u8,
        sensor_type: SensorType,
        offset: u8,
    }

    impl fmt::Display for EventOffset {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            decode_event(f, self.event_type, self.sensor_type, self.offset)
        }
    }

    #[test]
    fn test_system_event_offsets() {
        assert_eq!(
            SensorOffset {
                sensor_type: SensorType::SystemEvent,
                offset: 0x05,
            }
            .to_string(),
            "Timestamp Clock Sync"
        );
        assert_eq!(
            SensorOffset {
                sensor_type: SensorType::SystemEvent,
                offset: 0x00,
            }
            .to_string(),
            "System Reconfigured"
        );
    }

    #[test]
    fn test_acpi_power_state_offsets() {
        assert_eq!(
            SensorOffset {
                sensor_type: SensorType::SystemACPIPowerState,
                offset: 0x00,
            }
            .to_string(),
            "S0 / G0 'working'"
        );
        assert_eq!(
            SensorOffset {
                sensor_type: SensorType::SystemACPIPowerState,
                offset: 0x05,
            }
            .to_string(),
            "S5 / G2 'soft-off'"
        );
    }

    #[test]
    fn test_generic_threshold() {
        assert_eq!(
            GenericOffset {
                event_type: 0x01,
                offset: 0x09,
            }
            .to_string(),
            "Upper Critical - going high"
        );
    }

    #[test]
    fn test_decode_event() {
        // Sensor-specific event (0x6F)
        assert_eq!(
            EventOffset {
                event_type: 0x6F,
                sensor_type: SensorType::SystemEvent,
                offset: 0x05,
            }
            .to_string(),
            "Timestamp Clock Sync"
        );

        // Generic threshold event
        assert_eq!(
            EventOffset {
                event_type: 0x01,
                sensor_type: SensorType::Temperature,
                offset: 0x09,
            }
            .to_string(),
            "Upper Critical - going high"
        );
    }
}
