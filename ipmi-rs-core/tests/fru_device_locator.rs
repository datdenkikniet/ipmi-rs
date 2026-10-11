use ipmi_rs_core::connection::LogicalUnit;
use ipmi_rs_core::storage::sdr::record::{
    FruDevice, FruDeviceLocator, FruRecordKey, LogicalFruDevice, ParseError, PhysicalFruDevice,
    RecordContents,
};
use ipmi_rs_core::storage::sdr::Record;

// IPMI 2.0, Table 43-7: bytes 6-16, followed by the device ID string.
const LOGICAL_FRU: [u8; 14] = [
    0x20, 0x03, 0x90, 0xB0, 0x00, 0x10, 0x00, 0x0A, 0x02, 0x42, 0xC3, b'P', b'S', b'U',
];

fn parse_record(data: &[u8]) -> Result<FruDeviceLocator, ParseError> {
    let mut record = vec![0x01, 0x00, 0x51, 0x11, data.len() as u8];
    record.extend_from_slice(data);
    match Record::parse(&record)?.contents {
        RecordContents::FruDeviceLocator(locator) => Ok(locator),
        other => panic!("expected FRU device locator, got {other:?}"),
    }
}

#[test]
fn logical_flag_is_independent_of_fru_device_id() {
    for id in [0x03, 0x83] {
        let mut data = LOGICAL_FRU;
        data[1] = id;
        let locator = parse_record(&data).unwrap();
        let FruRecordKey {
            fru_device: FruDevice::Logical(LogicalFruDevice { fru_device_id }),
            ..
        } = locator.record_key
        else {
            panic!("logical FRU ID {id:#04x} was classified as physical");
        };
        assert_eq!(fru_device_id, id);
    }
}

#[test]
fn physical_flag_is_independent_of_i2c_address() {
    for address in [0x28, 0x52] {
        let mut data = LOGICAL_FRU;
        data[1] = address << 1;
        data[2] = 0x15; // Physical device, LUN 2, private bus 5.
        let locator = parse_record(&data).unwrap();
        let FruRecordKey {
            fru_device: FruDevice::Physical(PhysicalFruDevice { i2c_address }),
            ..
        } = locator.record_key
        else {
            panic!("physical I2C address {address:#04x} was classified as logical");
        };
        assert_eq!(i2c_address, address);
        assert_eq!(locator.record_key.private_bus_id, 5);
    }
}

#[test]
fn parses_channel_lun_and_record_body() {
    let locator = parse_record(&LOGICAL_FRU).unwrap();
    assert_eq!(locator.record_key.device_access_address, 0x10);
    assert_eq!(locator.record_key.channel_number, 0x0B);
    assert_eq!(locator.record_key.lun, LogicalUnit::Two);
    assert_eq!(locator.record_key.private_bus_id, 0);
    assert_eq!(locator.device_type, 0x10);
    assert_eq!(locator.device_type_modifier, 0x00);
    assert_eq!(locator.fru_entity_id, 0x0A);
    assert_eq!(locator.fru_entity_instance, 0x02);
    assert_eq!(locator.oem_reserved, 0x42);
    assert_eq!(locator.id_string().to_string(), "PSU");
}

#[test]
fn rejects_truncated_fixed_fields_without_panicking() {
    for len in 0..11 {
        assert!(
            matches!(
                parse_record(&LOGICAL_FRU[..len]),
                Err(ParseError::NotEnoughData)
            ),
            "accepted a {len}-byte FRU locator"
        );
    }
}

#[test]
fn accepts_minimum_length_with_empty_device_id() {
    let mut data = LOGICAL_FRU[..11].to_vec();
    data[10] = 0xC0;
    let locator = parse_record(&data).unwrap();
    assert_eq!(locator.id_string().to_string(), "");
}
