pub trait EventReadingTypeCode {
    const EVENT_READING_TYPE_CODE: u8;
    fn is_threshold(&self) -> bool;
    fn is_discrete(&self) -> bool;
    fn is_generic(&self) -> bool;
    fn is_sensor_specific(&self) -> bool;
    fn is_oem(&self) -> bool;
}

macro_rules! impl_event_reading_code {
    (threshold: $name:ident, $code:literal) => {
        impl_event_reading_code!($name, $code, true, false, false, false, false);
    };

    (generic: $name:ident, $code:literal) => {
        impl_event_reading_code!($name, $code, false, true, true, false, false);
    };

    (oem: $name:ident, $code:literal) => {
        impl_event_reading_code!($name, $code, false, false, false, false, true);
    };

    ($name:ident, $code:literal, $thrs:literal, $disc:literal, $generic:literal, $sensor_spec:literal, $oem:literal) => {
        impl EventReadingTypeCode for $name {
            const EVENT_READING_TYPE_CODE: u8 = $code;

            fn is_threshold(&self) -> bool {
                $thrs
            }

            fn is_discrete(&self) -> bool {
                $disc
            }

            fn is_generic(&self) -> bool {
                $generic
            }

            fn is_sensor_specific(&self) -> bool {
                $sensor_spec
            }

            fn is_oem(&self) -> bool {
                $oem
            }
        }
    };
}

macro_rules! event_reading_code {
    ($($disc_generic:ident => $disc_generic_val:literal,)*) => {
        $(
            pub struct $disc_generic(u16);

            impl $disc_generic {
                pub fn new(value: u16) -> Self {
                    Self(value)
                }

                pub fn value(&self) -> u16 {
                    self.0
                }
            }

            impl_event_reading_code!(generic: $disc_generic, $disc_generic_val);
        )*

        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum EventReadingTypeCodes {
            Unspecified,
            Threshold,
            DiscreteGeneric(u8),
            SensorSpecific,
            Oem(u8),
            Reserved(u8),
        }

        impl From<u8> for EventReadingTypeCodes {
            fn from(value: u8) -> Self {
                match value {
                    0x00 => Self::Unspecified,
                    0x01 => Self::Threshold,
                    0x02..=0x0C => Self::DiscreteGeneric(value),
                    0x6F => Self::SensorSpecific,
                    0x70..=0x7F => Self::Oem(value),
                    v => Self::Reserved(v),
                }
            }
        }

        pub enum EventReading {
            Threshold(Threshold),
            UsageState(UsageState),
            $($disc_generic($disc_generic),)*
        }
    };
}

pub struct Threshold;
impl_event_reading_code!(threshold: Threshold, 0x01);

pub struct UsageState;
impl_event_reading_code!(generic: UsageState, 0x02);

event_reading_code!(
    StateAssertion => 0x03,
    PredictiveFailure => 0x04,
    LimitExcess => 0x05,
    PerformanceMetric => 0x06,
    SeverityEvents => 0x07,
    DevicePresence => 0x08,
    DeviceEnabledStatus => 0x09,
    PowerState => 0x0A,
    RedundancyState => 0x0B,
    AcpiDevicePowerState => 0x0C,
);
