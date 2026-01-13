//! Sensor Type Codes
//!
//! Reference: IPMI 2.0 Specification, Table 42-3 "Sensor Type Codes"

use core::fmt;

macro_rules ! sensor_type {
    {
        pub enum SensorType {
            $(
                $(#[doc = $doc:literal])?
                $name:ident = $value:literal => $display:literal,
            )*
            [$reserved_range:pat],
            [$oem_reserved_range:pat],
        }
    } => {
        /// Sensor type codes as defined in IPMI 2.0 Specification, Table 42-3.
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(u8)]
        pub enum SensorType {
            $(
                $(#[doc = $doc])?
                $name = $value,
            )*
            /// Reserved sensor type
            Reserved(u8),
            /// OEM-defined sensor type (0xC0-0xFF)
            OemReserved(u8),
        }

        impl From<u8> for SensorType {
            fn from(value: u8) -> Self {
                match value {
                    $($value => Self::$name,)*
                    0 | $reserved_range => Self::Reserved(value),
                    $oem_reserved_range => Self::OemReserved(value),
                }
            }
        }

        impl From<SensorType> for u8 {
            fn from(value: SensorType) -> u8 {
                match value {
                    $(SensorType::$name => $value,)*
                    SensorType::Reserved(v) => v,
                    SensorType::OemReserved(v) => v,
                }
            }
        }

        impl TryFrom<&str> for SensorType {
            type Error = ();

            fn try_from(input: &str) -> Result<Self, Self::Error> {
                let to_lower = input.to_ascii_lowercase();

                $(
                    if stringify!($name).to_ascii_lowercase() == to_lower {
                        return Ok(SensorType::$name);
                    }
                )*

                Err(())
            }
        }

        impl fmt::Display for SensorType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(SensorType::$name => write!(f, $display),)*
                    SensorType::Reserved(v) => write!(f, "Reserved (0x{:02X})", v),
                    SensorType::OemReserved(v) => write!(f, "OEM (0x{:02X})", v),
                }
            }
        }
    }
}

sensor_type! {
    pub enum SensorType {
        Temperature = 0x01 => "Temperature",
        Voltage = 0x02 => "Voltage",
        Current = 0x03 => "Current",
        Fan = 0x04 => "Fan",
        ChassisIntrusion = 0x05 => "Physical Security",
        PlatformSecurityViolationAttempt = 0x06 => "Platform Security",
        Processor = 0x07 => "Processor",
        PowerSupply = 0x08 => "Power Supply",
        PowerUnit = 0x09 => "Power Unit",
        CoolingDevice = 0x0A => "Cooling Device",
        UnitsBasedSensor = 0x0B => "Other Units-based",
        Memory = 0x0C => "Memory",
        DriveSlotBay = 0x0D => "Drive Slot",
        PostMemoryResize = 0x0E => "POST Memory Resize",
        SystemFirmwareProgress = 0x0F => "System Firmware Progress",
        EventLoggingDisabled = 0x10 => "Event Logging Disabled",
        Watchdog1 = 0x11 => "Watchdog 1",
        SystemEvent = 0x12 => "System Event",
        CriticalInterrupt = 0x13 => "Critical Interrupt",
        ButtonOrSwitch = 0x14 => "Button/Switch",
        ModuleOrBoard = 0x15 => "Module/Board",
        MicroControllerOrCoprocessor = 0x16 => "Microcontroller",
        AddinCard = 0x17 => "Add-in Card",
        Chassis = 0x18 => "Chassis",
        ChipSet = 0x19 => "Chip Set",
        OtherFRU = 0x1A => "Other FRU",
        CableOrInterconnect = 0x1B => "Cable/Interconnect",
        Terminator = 0x1C => "Terminator",
        SystemBootOrRestartInitiated = 0x1D => "System Boot",
        BootError = 0x1E => "Boot Error",
        BaseOsBootOrInstallationStatus = 0x1F => "OS Boot",
        OsStopOrShutdown = 0x20 => "OS Stop/Shutdown",
        SlotOrConnector = 0x21 => "Slot/Connector",
        SystemACPIPowerState = 0x22 => "System ACPI Power State",
        Watchdog2 = 0x23 => "Watchdog 2",
        PlatformAlert = 0x24 => "Platform Alert",
        EntityPresence = 0x25 => "Entity Presence",
        MonitorAsicOrIc = 0x26 => "Monitor ASIC/IC",
        LAN = 0x27 => "LAN",
        ManagementSubSysHealth = 0x28 => "Management Subsystem Health",
        Battery = 0x29 => "Battery",
        SessionAudit = 0x2A => "Session Audit",
        VersionChange = 0x2B => "Version Change",
        FRUState = 0x2C => "FRU State",
        [0x2D..=0xBF],
        [0xC0..=0xFF],
    }
}
