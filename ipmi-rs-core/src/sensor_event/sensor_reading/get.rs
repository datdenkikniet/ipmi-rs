#[cfg(feature = "alloc")]
use crate::storage::sdr::record::SensorKey;
use crate::{
    connection::{Address, Channel, EncodeIpmiCommand, IpmiCommand, NetFn, NotEnoughData},
    storage::sdr::SensorNumber,
};

use super::RawSensorReading;

impl RawSensorReading {
    pub(crate) fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }

        let reading = data[0];

        // Bit indicates that all event messages are enabled => must negate result
        let all_event_messages_disabled = (data[1] & 0x80) != 0x80;

        // Bit indicates that sensor scanning is enabled => must negate result
        let scanning_disabled = (data[1] & 0x40) != 0x40;

        let reading_or_state_unavailable = (data[1] & 0x20) == 0x20;

        let offset_data_1 = data.get(2).copied();
        let offset_data_2 = data.get(3).copied();

        Some(Self {
            reading,
            all_event_messages_disabled,
            scanning_disabled,
            reading_or_state_unavailable,
            offset_data_1,
            offset_data_2,
        })
    }
}

pub struct GetSensorReading {
    sensor_number: SensorNumber,
    address: Address,
    channel: Channel,
}

impl GetSensorReading {
    pub fn new(sensor_number: SensorNumber, address: Address, channel: Channel) -> Self {
        Self {
            sensor_number,
            address,
            channel,
        }
    }

    #[cfg(feature = "alloc")]
    pub fn for_sensor_key(value: &SensorKey) -> Self {
        Self {
            sensor_number: value.sensor_number,
            address: Address(value.owner_id.into()),
            channel: value.owner_channel,
        }
    }
}

impl EncodeIpmiCommand for GetSensorReading {
    const NETFN: NetFn = NetFn::SensorEvent;
    const CMD: u8 = 0x2d;

    fn request_data_len(&self) -> usize {
        1
    }

    fn write_request_data(&self, data: &mut [u8]) {
        data[0] = self.sensor_number.get();
    }
}

impl IpmiCommand for GetSensorReading {
    type Output = RawSensorReading;

    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        RawSensorReading::parse(data).ok_or(NotEnoughData)
    }

    fn target(&self) -> Option<(Address, Channel)> {
        Some((self.address, self.channel))
    }
}
