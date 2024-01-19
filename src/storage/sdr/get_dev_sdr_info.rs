use std::marker::PhantomData;

use crate::{
    connection::{CompletionCode, IpmiCommand, LogicalUnit, Message, NetFn, ParseResponseError},
    log_vec, Loggable,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdrCount;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorCount;

trait FromOpValue {
    fn from(value: u8) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberOfSensors(pub u8);

impl FromOpValue for NumberOfSensors {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberOfSdrs(pub u8);

impl FromOpValue for NumberOfSdrs {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub struct DeviceSdrInfo<T> {
    pub operation_value: T,
    pub dynamic_population: bool,
    pub lun_0_has_sensors: bool,
    pub lun_1_has_sensors: bool,
    pub lun_2_has_sensors: bool,
    pub lun_3_has_sensors: bool,
    pub sensor_population_epoch: Option<u32>,
}

impl<T> DeviceSdrInfo<T> {
    pub fn lun_has_sensors(&self, lun: LogicalUnit) -> bool {
        match lun {
            LogicalUnit::Zero => self.lun_0_has_sensors,
            LogicalUnit::One => self.lun_1_has_sensors,
            LogicalUnit::Two => self.lun_2_has_sensors,
            LogicalUnit::Three => self.lun_3_has_sensors,
        }
    }

    fn partial_log(&self, mut log: Vec<crate::fmt::LogItem>) -> Vec<crate::fmt::LogItem> {
        let mut luns_with_sensors = Vec::new();
        if self.lun_0_has_sensors {
            luns_with_sensors.push(0);
        }
        if self.lun_1_has_sensors {
            luns_with_sensors.push(1);
        }
        if self.lun_2_has_sensors {
            luns_with_sensors.push(2);
        }
        if self.lun_3_has_sensors {
            luns_with_sensors.push(3);
        }

        log.push((1, "LUNs with sensors", format!("{:?}", luns_with_sensors)).into());

        if let Some(epoch) = self.sensor_population_epoch {
            log.push((1, "Sensor pop. epoch", format!("0x{epoch:04X}")).into());
        }

        log
    }

    fn parse(data: &[u8]) -> Option<Self>
    where
        T: FromOpValue,
    {
        if data.len() < 2 {
            return None;
        }

        let op_value = data[0];
        let dynamic_population = (data[1] & 0x80) == 0x80;

        let lun_3_has_sensors = (data[1] & 0x08) == 0x08;
        let lun_2_has_sensors = (data[1] & 0x04) == 0x04;
        let lun_1_has_sensors = (data[1] & 0x02) == 0x02;
        let lun_0_has_sensors = (data[1] & 0x01) == 0x01;

        let sensor_population_epoch = if dynamic_population && data.len() < 6 {
            return None;
        } else if dynamic_population {
            Some(u32::from_le_bytes([data[2], data[3], data[4], data[5]]))
        } else {
            None
        };

        Some(Self {
            operation_value: T::from(op_value),
            dynamic_population,
            lun_0_has_sensors,
            lun_1_has_sensors,
            lun_2_has_sensors,
            lun_3_has_sensors,
            sensor_population_epoch,
        })
    }
}

impl Loggable for DeviceSdrInfo<NumberOfSdrs> {
    fn into_log(&self) -> Vec<crate::fmt::LogItem> {
        let log = log_vec![
            (0, "Device SDR information"),
            (1, "Number of SDRs", self.operation_value.0)
        ];
        self.partial_log(log)
    }
}

impl Loggable for DeviceSdrInfo<NumberOfSensors> {
    fn into_log(&self) -> Vec<crate::fmt::LogItem> {
        let log = log_vec![
            (0, "Device SDR information"),
            (1, "Number of sensors", self.operation_value.0)
        ];
        self.partial_log(log)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GetDeviceSdrInfo<T> {
    _phantom: PhantomData<T>,
}

impl<T> GetDeviceSdrInfo<T> {
    pub fn new(_: T) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl From<GetDeviceSdrInfo<SdrCount>> for Message {
    fn from(_: GetDeviceSdrInfo<SdrCount>) -> Self {
        Message::new_request(NetFn::SensorEvent, 0x20, vec![0x01])
    }
}

impl From<GetDeviceSdrInfo<SensorCount>> for Message {
    fn from(_: GetDeviceSdrInfo<SensorCount>) -> Self {
        Message::new_request(NetFn::SensorEvent, 0x20, vec![0x01])
    }
}

impl IpmiCommand for GetDeviceSdrInfo<SdrCount> {
    type Output = DeviceSdrInfo<NumberOfSdrs>;

    type Error = ();

    fn parse_response(
        completion_code: CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        DeviceSdrInfo::parse(data).ok_or(ParseResponseError::NotEnoughData)
    }
}

impl IpmiCommand for GetDeviceSdrInfo<SensorCount> {
    type Output = DeviceSdrInfo<NumberOfSensors>;

    type Error = ();

    fn parse_response(
        completion_code: CompletionCode,
        data: &[u8],
    ) -> Result<Self::Output, ParseResponseError<Self::Error>> {
        Self::check_cc_success(completion_code)?;

        DeviceSdrInfo::parse(data).ok_or(ParseResponseError::NotEnoughData)
    }
}
