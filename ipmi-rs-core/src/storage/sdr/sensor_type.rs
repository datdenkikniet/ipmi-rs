macro_rules ! sensor_type {
    {
        pub enum SensorType {
            $($name:ident = $value:literal,)*
            [$reserved_range:pat],
            [$oem_reserved_range:pat],
        }
    } => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(u8)]
        pub enum SensorType {
            $($name = $value,)*
            Reserved(u8),
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
    }
}

sensor_type! {
    pub enum SensorType {
        Temperature = 0x01,
        Voltage = 0x02,
        Current = 0x03,
        Fan = 0x04,
        ChassisIntrusion = 0x05,
        PlatformSecurityViolationAttempt = 0x06,
        Processor = 0x07,
        PowerSupply = 0x08,
        PowerUnit = 0x09,
        CoolingDevice = 0x0A,
        UnitsBasedSensor = 0x0B,
        Memory = 0x0C,
        DriveSlotBay = 0x0D,
        PostMemoryResize = 0x0E,
        SystemFirmwareProgress = 0x0F,
        EventLoggingDisabled = 0x10,
        Watchdog1 = 0x11,
        SystemEvent = 0x12,
        CriticalInterrupt = 0x13,
        ButtonOrSwitch = 0x14,
        ModuleOrBoard = 0x15,
        MicroControllerOrCoprocessor = 0x16,
        AddinCard = 0x17,
        Chassis = 0x18,
        ChipSet = 0x19,
        OtherFRU = 0x1A,
        CableOrInterconnect = 0x1B,
        Terminator = 0x1C,
        SystemBootOrRestartInitiated = 0x1D,
        BootError = 0x1E,
        BaseOsBootOrInstallationStatus = 0x1F,
        OsStopOrShutdown = 0x20,
        SlotOrConnector = 0x21,
        SystemACPIPowerState = 0x22,
        Watchdog2 = 0x23,
        PlatformAlert = 0x24,
        EntityPresence = 0x25,
        MonitorAsicOrIc = 0x26,
        LAN = 0x27,
        ManagementSubSysHealth = 0x28,
        Battery = 0x29,
        SessionAudit = 0x2A,
        VersionChange = 0x2B,
        FRUState = 0x2C,
        [0x2D..=0xBF],
        [0xC0..=0xFF],
    }
}
