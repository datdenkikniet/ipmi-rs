//! SEL (System Event Log) tool example
//!
//! This example demonstrates reading and clearing SEL entries.
//!
//! Usage:
//!   # List all SEL entries
//!   cargo run --example sel
//!
//!   # Clear the SEL
//!   cargo run --example sel -- --clear
//!
//!   # Use RMCP connection
//!   cargo run --example sel -- -c 'rmcp://user:pass@192.168.1.100'

mod common;

use std::collections::HashMap;
use std::fmt;

use clap::Parser;
use common::IpmiConnectionEnum;
use ipmi_rs::{
    storage::{
        sdr::{
            record::{
                FullSensorRecord, InstancedSensor, RecordContents, SensorKey as SdrSensorKey,
                SensorOwner, WithSensorRecordCommon,
            },
            EventData, SensorType, Unit,
        },
        sel::{
            ClearSel, Entry, ErasureProgress, EventGenerator, GetSelEntry, GetSelInfo,
            RecordId as SelRecordId, ReserveSel, SelCommand,
        },
    },
    IpmiError,
};

/// Sensor lookup key: (owner_id, channel, owner_lun, sensor_number)
type SensorKey = (u8, u8, u8, u8);

/// Sensor info for displaying readings with proper units
#[derive(Clone)]
struct SensorInfo {
    name: String,
    /// Conversion parameters from Full Sensor Record (if available)
    conversion: Option<SensorConversion>,
}

/// Conversion parameters to convert raw reading to actual value with units
#[derive(Clone)]
struct SensorConversion {
    m: i16,
    b: i16,
    b_exponent: i8,
    result_exponent: i8,
    base_unit: Unit,
    data_format: ipmi_rs::storage::sdr::record::DataFormat,
}

struct EventDescription<'a> {
    entry: &'a Entry,
}

impl fmt::Display for EventDescription<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.entry.event_description(f)
    }
}

impl SensorConversion {
    fn from_full_sensor(record: &FullSensorRecord) -> Option<Self> {
        Some(Self {
            m: record.m,
            b: record.b,
            b_exponent: record.b_exponent,
            result_exponent: record.result_exponent,
            base_unit: record.common().sensor_units.base_unit,
            data_format: record
                .analog_data_format
                .unwrap_or(ipmi_rs::storage::sdr::record::DataFormat::Unsigned),
        })
    }

    /// Convert raw reading to actual value with units
    fn convert(&self, raw: u8) -> String {
        let m = self.m as f32;
        let b = self.b as f32 * 10f32.powf(self.b_exponent as f32);
        let result_mul = 10f32.powf(self.result_exponent as f32);

        let value = match self.data_format {
            ipmi_rs::storage::sdr::record::DataFormat::Unsigned => raw as i16,
            ipmi_rs::storage::sdr::record::DataFormat::TwosComplement => raw as i8 as i16,
            ipmi_rs::storage::sdr::record::DataFormat::OnesComplement => {
                if (raw & 0x80) == 0 {
                    raw as i16
                } else {
                    let magnitude = (!raw & 0xFF) as i16;
                    if magnitude == 0 {
                        0
                    } else {
                        -magnitude
                    }
                }
            }
        } as f32;

        let converted = (m * value + b) * result_mul;
        self.base_unit.display(true, converted)
    }
}

/// Sensor lookup table: maps (sensor_type, sensor_number) to sensor info
type SensorLookup = HashMap<SensorKey, SensorInfo>;

fn sensor_lookup_key_from_sdr(key: &SdrSensorKey) -> SensorKey {
    (
        u8::from(key.owner_id),
        key.owner_channel.value(),
        key.owner_lun.value(),
        key.sensor_number.get(),
    )
}

fn sensor_lookup_key_from_generator(generator: &EventGenerator, sensor_number: u8) -> SensorKey {
    match generator {
        EventGenerator::RqSAAndLun {
            i2c_addr,
            channel_number,
            lun,
        } => (
            u8::from(SensorOwner::I2C(*i2c_addr)),
            channel_number.value(),
            lun.value(),
            sensor_number,
        ),
        EventGenerator::SoftwareId {
            software_id,
            channel_number,
        } => (
            u8::from(SensorOwner::System(*software_id)),
            channel_number.value(),
            // Software ID entries do not expose LUN in EventGenerator; assume 0.
            0,
            sensor_number,
        ),
    }
}

/// Format event data with proper unit conversion
fn format_event_data_with_units(event_data: &EventData, conv: &SensorConversion) -> Option<String> {
    use ipmi_rs::storage::sdr::{EventData2Type, EventData3Type};

    let mut parts = Vec::new();

    // Convert reading if present
    match event_data.data2_type {
        EventData2Type::TriggerReading(value) => {
            parts.push(format!("reading={}", conv.convert(value.get())));
        }
        EventData2Type::OemCode(value) => {
            parts.push(format!("oem2=0x{:02X}", value.get()));
        }
        EventData2Type::SensorSpecific(value) => {
            parts.push(format!("ext2=0x{:02X}", value.get()));
        }
        EventData2Type::Unspecified => {}
    }

    // Convert threshold if present
    match event_data.data3_type {
        EventData3Type::TriggerThreshold(value) => {
            parts.push(format!("threshold={}", conv.convert(value.get())));
        }
        EventData3Type::OemCode(value) => {
            parts.push(format!("oem3=0x{:02X}", value.get()));
        }
        EventData3Type::SensorSpecific(value) => {
            parts.push(format!("ext3=0x{:02X}", value.get()));
        }
        EventData3Type::Unspecified => {}
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

/// Build a sensor lookup table from SDR records.
fn build_sensor_lookup(ipmi: &mut IpmiConnectionEnum) -> SensorLookup {
    let mut lookup = HashMap::new();

    log::info!("Loading SDR records for sensor name lookup...");

    for record in ipmi.sdrs() {
        let (_sensor_type, _sensor_number, sensor_key) = match &record.contents {
            RecordContents::FullSensor(full) => (
                u8::from(*full.ty()),
                full.common().key.sensor_number.get(),
                &full.common().key,
            ),
            RecordContents::CompactSensor(compact) => (
                u8::from(*compact.ty()),
                compact.common().key.sensor_number.get(),
                &compact.common().key,
            ),
            RecordContents::EventOnlySensor(event) => (
                u8::from(event.ty),
                event.key.sensor_number.get(),
                &event.key,
            ),
            _ => continue,
        };
        let name = match record.contents.id() {
            Some(id) => id.to_string(),
            None => continue,
        };

        // Get conversion parameters from Full Sensor Records
        let conversion = match &record.contents {
            RecordContents::FullSensor(full) => SensorConversion::from_full_sensor(full),
            _ => None,
        };

        lookup.insert(
            sensor_lookup_key_from_sdr(sensor_key),
            SensorInfo { name, conversion },
        );
    }

    log::info!("Loaded {} sensor names from SDR", lookup.len());
    lookup
}

/// Get manufacturer name from IANA enterprise number.
///
/// Reference: https://www.iana.org/assignments/enterprise-numbers/
fn manufacturer_name(id: u32) -> &'static str {
    match id {
        0x000002 => "IBM",
        0x000009 => "Cisco",
        0x00000B => "HP",
        0x00000E => "Fujitsu Siemens",
        0x000028 => "Dell",
        0x000137 => "Fujitsu",
        0x000157 => "Intel",
        0x00028A => "Nokia",
        0x002A7C => "Supermicro",
        0x00A2B7 => "Quanta",
        _ => "Unknown",
    }
}

/// Result of decoding OEM data
#[derive(Debug)]
enum OemDecodeResult {
    /// Printable text characters
    Text(String),
    /// Message header/separator (start of new message)
    Header,
    /// End of message (contains control chars like CR/LF)
    EndOfMessage(String),
    /// Could not decode
    Unknown,
}

/// Result of decoding OEM non-timestamped data
#[derive(Debug)]
enum OemNonTimestampedResult {
    /// Text fragment with sequence number (sequence, text)
    TextFragment { sequence: u8, text: String },
    /// Could not decode as text
    Unknown,
}

/// Try to decode OEM non-timestamped data as ASCII text.
/// Common format: byte 0 = sub-type, byte 1 = sequence, bytes 2-12 = ASCII text
fn try_decode_oem_nontimestamped_text(data: &[u8; 13]) -> OemNonTimestampedResult {
    let sequence = data[1];

    // Try to extract ASCII text from bytes 2-12
    let mut text = String::new();
    for &byte in &data[2..] {
        if byte == 0 {
            // Null terminator - end of string
            break;
        } else if byte.is_ascii_graphic() || byte == b' ' {
            text.push(byte as char);
        } else if byte == 0x0D || byte == 0x0A {
            // CR/LF - treat as space
            if !text.ends_with(' ') {
                text.push(' ');
            }
        } else {
            // Non-printable character - might not be text
            return OemNonTimestampedResult::Unknown;
        }
    }

    if text.is_empty() {
        OemNonTimestampedResult::Unknown
    } else {
        OemNonTimestampedResult::TextFragment { sequence, text }
    }
}

/// Try to decode OEM data as ASCII text (for manufacturers like Fujitsu)
/// Fujitsu format: byte 0 = sequence, bytes 1,3 = ASCII chars, bytes 2,4,5 = padding
fn try_decode_oem_text(data: &[u8; 6]) -> OemDecodeResult {
    // Check for header/separator pattern (all zeros or specific markers)
    if data[0] == 0x00 && data[1] == 0x00 && data[2] == 0x00 {
        return OemDecodeResult::Header;
    }

    let char1 = data[1];
    let char2 = data[3];

    // Check for control characters (CR=0x0D, LF=0x0A) which indicate end of message
    let is_control1 = char1 == 0x0D || char1 == 0x0A;
    let is_control2 = char2 == 0x0D || char2 == 0x0A;

    if is_control1 || is_control2 {
        let mut s = String::new();
        if char1.is_ascii_graphic() || char1 == b' ' {
            s.push(char1 as char);
        }
        if char2.is_ascii_graphic() || char2 == b' ' {
            s.push(char2 as char);
        }
        return OemDecodeResult::EndOfMessage(s);
    }

    // Check if they look like printable ASCII
    if char1.is_ascii_graphic() || char1 == b' ' {
        if char2.is_ascii_graphic() || char2 == b' ' || char2 == 0 {
            let mut s = String::new();
            s.push(char1 as char);
            if char2 != 0 {
                s.push(char2 as char);
            }
            return OemDecodeResult::Text(s);
        }
    }

    OemDecodeResult::Unknown
}

#[derive(Parser)]
struct CliOpts {
    #[clap(flatten)]
    common: common::CommonOpts,

    /// Clear all SEL entries
    #[clap(long)]
    clear: bool,
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::formatted_builder()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or("info".to_string()))
        .init();

    let opts = CliOpts::parse();
    let mut ipmi = opts.common.get_connection()?;

    // Get SEL info first
    log::info!("Getting SEL info...");
    let info = ipmi.send_recv(GetSelInfo).expect("Failed to get SEL info");

    log::info!(
        "SEL Version: {}.{}, Entries: {}, Free bytes: {}",
        info.version_maj,
        info.version_min,
        info.entries,
        info.bytes_free
    );

    if info.overflow {
        log::warn!("SEL overflow flag is set!");
    }

    if opts.clear {
        // Clear SEL
        if !info.supported_cmds.contains(&SelCommand::Clear) {
            log::error!("SEL Clear command is not supported by this BMC");
            return Ok(());
        }

        if !info.supported_cmds.contains(&SelCommand::Reserve) {
            log::error!("SEL Reserve command is not supported by this BMC");
            return Ok(());
        }

        log::info!("Reserving SEL...");
        let reservation_id = ipmi.send_recv(ReserveSel).expect("Failed to reserve SEL");

        log::info!(
            "Got reservation ID: 0x{:04X}, initiating clear...",
            reservation_id.get()
        );

        let progress = ipmi
            .send_recv(ClearSel::initiate(Some(reservation_id)))
            .expect("Failed to initiate SEL clear");

        match progress {
            ErasureProgress::Completed => {
                log::info!("SEL cleared successfully!");
            }
            ErasureProgress::InProgress => {
                log::info!("SEL clear initiated, checking status...");
                // Poll for completion
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    match ipmi.send_recv(ClearSel::get_status(Some(reservation_id))) {
                        Ok(ErasureProgress::Completed) => {
                            log::info!("SEL cleared successfully!");
                            break;
                        }
                        Ok(ErasureProgress::InProgress) => {
                            log::debug!("Still in progress...");
                            continue;
                        }
                        Err(e) => {
                            log::error!("Failed to get clear status: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }
    } else {
        // List SEL entries
        if info.entries == 0 {
            log::info!("SEL is empty");
            return Ok(());
        }

        // Build sensor lookup table from SDR
        let sensor_lookup = build_sensor_lookup(&mut ipmi);

        log::info!("Reading {} SEL entries...", info.entries);

        let mut record_id = SelRecordId::FIRST;
        let mut count = 0u32;
        let mut oem_text_buffer = String::new();
        let mut oem_nts_buffer = String::new(); // OEM non-timestamped text buffer
        let mut oem_nts_last_seq: Option<u8> = None;

        loop {
            match ipmi.send_recv(GetSelEntry::new(None, record_id)) {
                Ok(entry_info) => {
                    count += 1;
                    print_entry(
                        count,
                        &entry_info.entry,
                        &mut oem_text_buffer,
                        &mut oem_nts_buffer,
                        &mut oem_nts_last_seq,
                        &sensor_lookup,
                    );

                    if entry_info.next_entry.is_last() {
                        break;
                    }
                    record_id = entry_info.next_entry;
                }
                Err(IpmiError::Failed {
                    completion_code, ..
                }) => {
                    log::error!(
                        "Failed to get SEL entry: completion code {:?}",
                        completion_code
                    );
                    break;
                }
                Err(e) => {
                    log::error!("Failed to get SEL entry: {:?}", e);
                    break;
                }
            }
        }

        // Flush any remaining OEM text
        flush_oem_buffer(&mut oem_text_buffer);
        flush_oem_nts_buffer(&mut oem_nts_buffer);

        log::info!("Read {} SEL entries", count);
    }

    Ok(())
}

fn flush_oem_buffer(oem_text_buffer: &mut String) {
    if !oem_text_buffer.is_empty() {
        log::info!("  >>> OEM Message: \"{}\"", oem_text_buffer);
        oem_text_buffer.clear();
    }
}

fn flush_oem_nts_buffer(oem_nts_buffer: &mut String) {
    if !oem_nts_buffer.is_empty() {
        log::info!("  >>> OEM Message: \"{}\"", oem_nts_buffer.trim());
        oem_nts_buffer.clear();
    }
}

fn print_entry(
    num: u32,
    entry: &Entry,
    oem_text_buffer: &mut String,
    oem_nts_buffer: &mut String,
    oem_nts_last_seq: &mut Option<u8>,
    sensor_lookup: &SensorLookup,
) {
    match entry {
        Entry::System {
            record_id,
            timestamp,
            generator_id,
            sensor_type,
            sensor_number,
            event_direction,
            event_type,
            event_data,
            ..
        } => {
            // Flush any accumulated OEM text
            flush_oem_buffer(oem_text_buffer);
            flush_oem_nts_buffer(oem_nts_buffer);
            *oem_nts_last_seq = None;

            // Convert raw sensor type to enum for display and decoding
            let sensor = SensorType::from(*sensor_type);

            // Look up sensor info from SDR
            let sensor_info = sensor_lookup.get(&sensor_lookup_key_from_generator(
                generator_id,
                *sensor_number,
            ));
            let sensor_name = sensor_info.map(|s| s.name.as_str()).unwrap_or("");

            let sensor_display = if sensor_name.is_empty() {
                format!("{} #{}", sensor, sensor_number)
            } else {
                format!("{} ({})", sensor_name, sensor)
            };

            let parsed_event_data = event_data;

            // Try to decode the event offset to a human-readable description
            let event_description = {
                let desc = EventDescription { entry }.to_string();
                if desc.is_empty() {
                    None
                } else {
                    Some(desc)
                }
            };

            // Format additional event data info with unit conversion if available
            let extra_data = if let Some(info) = sensor_info {
                if let Some(ref conv) = info.conversion {
                    // Use proper unit conversion for readings
                    format_event_data_with_units(&parsed_event_data, conv)
                } else {
                    let data = parsed_event_data.to_string();
                    if data.is_empty() {
                        None
                    } else {
                        Some(data)
                    }
                }
            } else {
                let data = parsed_event_data.to_string();
                if data.is_empty() {
                    None
                } else {
                    Some(data)
                }
            };

            if let Some(desc) = event_description {
                if let Some(extra) = extra_data {
                    log::info!(
                        "#{:4} | ID: 0x{:04X} | {} | {} | {:?} | {} ({})",
                        num,
                        record_id.value(),
                        timestamp,
                        sensor_display,
                        event_direction,
                        desc,
                        extra
                    );
                } else {
                    log::info!(
                        "#{:4} | ID: 0x{:04X} | {} | {} | {:?} | {}",
                        num,
                        record_id.value(),
                        timestamp,
                        sensor_display,
                        event_direction,
                        desc
                    );
                }
            } else {
                // No event description available - show raw data with parsed info
                if let Some(extra) = extra_data {
                    log::info!(
                        "#{:4} | ID: 0x{:04X} | {} | {} | Type: 0x{:02X} | {:?} | offset={} ({})",
                        num,
                        record_id.value(),
                        timestamp,
                        sensor_display,
                        event_type,
                        event_direction,
                        parsed_event_data.offset,
                        extra
                    );
                } else {
                    log::info!(
                        "#{:4} | ID: 0x{:04X} | {} | {} | Type: 0x{:02X} | {:?} | offset={}",
                        num,
                        record_id.value(),
                        timestamp,
                        sensor_display,
                        event_type,
                        event_direction,
                        parsed_event_data.offset
                    );
                }
            }
        }
        Entry::OemTimestamped {
            record_id,
            ty,
            timestamp,
            manufacturer_id,
            data,
        } => {
            match try_decode_oem_text(data) {
                OemDecodeResult::Text(text) => {
                    oem_text_buffer.push_str(&text);
                    log::debug!(
                        "#{:4} | ID: 0x{:04X} | {} | {} OEM (0x{:02X}) | \"{}\"",
                        num,
                        record_id.value(),
                        timestamp,
                        manufacturer_name(*manufacturer_id),
                        ty,
                        text
                    );
                }
                OemDecodeResult::EndOfMessage(text) => {
                    oem_text_buffer.push_str(&text);
                    flush_oem_buffer(oem_text_buffer);
                }
                OemDecodeResult::Header => {
                    // New message starting, flush old one
                    flush_oem_buffer(oem_text_buffer);
                    log::debug!(
                        "#{:4} | ID: 0x{:04X} | {} | {} OEM (0x{:02X}) | [message header]",
                        num,
                        record_id.value(),
                        timestamp,
                        manufacturer_name(*manufacturer_id),
                        ty
                    );
                }
                OemDecodeResult::Unknown => {
                    // Flush any accumulated OEM text
                    flush_oem_buffer(oem_text_buffer);

                    log::info!(
                        "#{:4} | ID: 0x{:04X} | {} | {} OEM (0x{:02X}) | Data: {:02X?}",
                        num,
                        record_id.value(),
                        timestamp,
                        manufacturer_name(*manufacturer_id),
                        ty,
                        data
                    );
                }
            }
        }
        Entry::OemNotTimestamped {
            record_id,
            ty,
            data,
        } => {
            // Flush any accumulated OEM timestamped text (different format)
            flush_oem_buffer(oem_text_buffer);

            // Try to decode as text
            match try_decode_oem_nontimestamped_text(data) {
                OemNonTimestampedResult::TextFragment { sequence, text } => {
                    // Check if this is a new message (sequence restarted or gap)
                    if sequence == 0 || oem_nts_last_seq.map_or(true, |s| sequence != s + 1) {
                        // Flush previous message if any
                        flush_oem_nts_buffer(oem_nts_buffer);
                    }

                    oem_nts_buffer.push_str(&text);
                    *oem_nts_last_seq = Some(sequence);

                    log::debug!(
                        "#{:4} | ID: 0x{:04X} | OEM (0x{:02X}) seq={} | \"{}\"",
                        num,
                        record_id.value(),
                        ty,
                        sequence,
                        text
                    );
                }
                OemNonTimestampedResult::Unknown => {
                    // Flush any accumulated text
                    flush_oem_nts_buffer(oem_nts_buffer);
                    *oem_nts_last_seq = None;

                    log::info!(
                        "#{:4} | ID: 0x{:04X} | OEM Non-timestamped (0x{:02X}) | Data: {:02X?}",
                        num,
                        record_id.value(),
                        ty,
                        data
                    );
                }
            }
        }
    }
}
