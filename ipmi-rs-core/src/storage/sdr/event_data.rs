//! Event Data Field Decoding
//!
//! Reference: IPMI 2.0 Specification, Section 29.7 "Event Data Field Formats"
//!
//! This module provides decoding for the 3-byte event data field in SEL entries.

use core::fmt;

use nonmax::NonMaxU8;

/// Event data byte 2 availability/format (from event_data\[0\] bits \[5:4\])
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventData2Type {
    /// Unspecified (00b)
    Unspecified,
    /// Trigger reading in byte 2 (01b)
    TriggerReading(NonMaxU8),
    /// OEM code in byte 2 (10b)
    OemCode(NonMaxU8),
    /// Sensor-specific extension code in byte 2 (11b)
    SensorSpecific(NonMaxU8),
}

/// Event data byte 3 availability/format (from event_data\[0\] bits \[7:6\])
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventData3Type {
    /// Unspecified (00b)
    Unspecified,
    /// Trigger threshold value in byte 3 (01b)
    TriggerThreshold(NonMaxU8),
    /// OEM code in byte 3 (10b)
    OemCode(NonMaxU8),
    /// Sensor-specific extension code in byte 3 (11b)
    SensorSpecific(NonMaxU8),
}

impl EventData2Type {
    /// Returns true when the data byte does not carry useful data.
    pub fn is_unspecified(&self) -> bool {
        matches!(self, Self::Unspecified)
    }

    fn parse(kind: u8, value: u8) -> Self {
        let Some(value) = NonMaxU8::new(value) else {
            return Self::Unspecified;
        };

        match kind {
            0b01 => Self::TriggerReading(value),
            0b10 => Self::OemCode(value),
            0b11 => Self::SensorSpecific(value),
            _ => Self::Unspecified,
        }
    }
}

impl fmt::Display for EventData2Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TriggerReading(value) => write!(f, "reading={}", value.get()),
            Self::OemCode(value) => write!(f, "oem2=0x{:02X}", value.get()),
            Self::SensorSpecific(value) => write!(f, "ext2=0x{:02X}", value.get()),
            Self::Unspecified => Ok(()),
        }
    }
}

impl EventData3Type {
    /// Returns true when the data byte does not carry useful data.
    pub fn is_unspecified(&self) -> bool {
        matches!(self, Self::Unspecified)
    }

    fn parse(kind: u8, value: u8) -> Self {
        let Some(value) = NonMaxU8::new(value) else {
            return Self::Unspecified;
        };

        match kind {
            0b01 => Self::TriggerThreshold(value),
            0b10 => Self::OemCode(value),
            0b11 => Self::SensorSpecific(value),
            _ => Self::Unspecified,
        }
    }
}

impl fmt::Display for EventData3Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TriggerThreshold(value) => write!(f, "threshold={}", value.get()),
            Self::OemCode(value) => write!(f, "oem3=0x{:02X}", value.get()),
            Self::SensorSpecific(value) => write!(f, "ext3=0x{:02X}", value.get()),
            Self::Unspecified => Ok(()),
        }
    }
}

/// Decoded event data from a SEL entry.
///
/// Reference: IPMI 2.0 Specification, Section 29.7
#[derive(Debug, Clone, PartialEq)]
pub struct EventData {
    /// Event offset (bits \[3:0\] of byte 1)
    pub offset: u8,
    /// Event data byte 2 type
    pub data2_type: EventData2Type,
    /// Event data byte 3 type
    pub data3_type: EventData3Type,
}

impl EventData {
    /// Parse event data from the 3-byte event data field.
    pub fn parse(data: &[u8; 3]) -> Self {
        let offset = data[0] & 0x0F;
        let data2_type = EventData2Type::parse((data[0] >> 4) & 0x03, data[1]);
        let data3_type = EventData3Type::parse((data[0] >> 6) & 0x03, data[2]);

        Self {
            offset,
            data2_type,
            data3_type,
        }
    }
}

impl fmt::Display for EventData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if !self.data2_type.is_unspecified() {
            parts.push(self.data2_type.to_string());
        }
        if !self.data3_type.is_unspecified() {
            parts.push(self.data3_type.to_string());
        }

        write!(f, "{}", parts.join(", "))
    }
}

/// Decode Power Supply sensor-specific event data.
///
/// Reference: IPMI 2.0 Specification, Table 42-3, Sensor Type 08h
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_data() {
        // Test with trigger reading and threshold
        let data = EventData::parse(&[0x59, 0x42, 0x50]); // offset 9, trigger reading, trigger threshold
        assert_eq!(data.offset, 0x09);
        assert_eq!(
            data.data2_type,
            EventData2Type::TriggerReading(NonMaxU8::new(0x42).unwrap())
        );
        assert_eq!(
            data.data3_type,
            EventData3Type::TriggerThreshold(NonMaxU8::new(0x50).unwrap())
        );
    }

    #[test]
    fn test_format_trigger_data() {
        let data = EventData::parse(&[0x59, 0x42, 0x50]);
        assert_eq!(data.to_string(), "reading=66, threshold=80");
    }

    #[test]
    fn test_unspecified_data() {
        let data = EventData::parse(&[0x00, 0xFF, 0xFF]);
        assert_eq!(data.to_string(), "");
    }
}
