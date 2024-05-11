use crate::{
    connection::{Address, Channel, CompletionCode, IpmiCommand, Message, ParseResponseError},
    storage::sdr::record::{SensorKey, SensorNumber},
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

    pub fn for_sensor_key(value: &SensorKey) -> Self {
        Self {
            sensor_number: value.sensor_number,
            address: Address(value.owner_id.into()),
            channel: value.owner_channel,
        }
    }
}

impl From<GetSensorReading> for Message {
    fn from(value: GetSensorReading) -> Self {
        Message::new_request(
            crate::connection::NetFn::SensorEvent,
            0x2D,
            vec![value.sensor_number.get()],
        )
    }
}

impl IpmiCommand for GetSensorReading {
    type Output = RawSensorReading;

    type Error = ();

    fn parse_response(
        completion_code: CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        RawSensorReading::parse(data).ok_or(ParseResponseError::NotEnoughData)
    }

    fn target(&self) -> Option<(Address, Channel)> {
        Some((self.address, self.channel))
    }
}
