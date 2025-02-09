mod get;
pub use get::GetSensorReading;

use crate::storage::sdr::event_reading_type_code::Threshold;

pub trait FromSensorReading {
    type Sensor;

    fn from(sensor: &Self::Sensor, reading: &RawSensorReading) -> Self;
}

#[derive(Debug, Clone, Copy)]
pub struct RawSensorReading {
    reading: u8,
    all_event_messages_disabled: bool,
    scanning_disabled: bool,
    reading_or_state_unavailable: bool,
    offset_data_1: Option<u8>,
    #[allow(unused)]
    offset_data_2: Option<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct ThresholdStatus {
    pub at_or_above_non_recoverable: bool,
    pub at_or_above_upper_critical: bool,
    pub at_or_above_upper_non_critical: bool,
    pub at_or_below_lower_non_recoverable: bool,
    pub at_or_below_lower_critical: bool,
    pub at_or_below_lower_non_critical: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ThresholdReading {
    pub all_event_messages_disabled: bool,
    pub scanning_disabled: bool,
    pub reading: Option<u8>,
    pub threshold_status: Option<ThresholdStatus>,
}

impl From<&RawSensorReading> for ThresholdReading {
    fn from(in_reading: &RawSensorReading) -> Self {
        let threshold_status = if in_reading.reading_or_state_unavailable {
            None
        } else {
            in_reading.offset_data_1.map(|d| ThresholdStatus {
                at_or_above_non_recoverable: (d & 0x20) == 0x20,
                at_or_above_upper_critical: (d & 0x10 == 0x10),
                at_or_above_upper_non_critical: (d & 0x08) == 0x08,
                at_or_below_lower_non_recoverable: (d & 0x04) == 0x04,
                at_or_below_lower_critical: (d & 0x20) == 0x20,
                at_or_below_lower_non_critical: (d & 0x01) == 0x01,
            })
        };

        let reading = if in_reading.reading_or_state_unavailable {
            None
        } else {
            Some(in_reading.reading)
        };

        Self {
            all_event_messages_disabled: in_reading.all_event_messages_disabled,
            scanning_disabled: in_reading.scanning_disabled,
            reading,
            threshold_status,
        }
    }
}

impl FromSensorReading for ThresholdReading {
    type Sensor = Threshold;

    fn from(_: &Self::Sensor, in_reading: &RawSensorReading) -> Self {
        in_reading.into()
    }
}
