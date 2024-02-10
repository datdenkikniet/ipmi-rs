mod authentication;
pub(crate) use authentication::AuthenticationAlgorithm;

mod confidentiality;
pub(crate) use confidentiality::ConfidentialityAlgorithm;

mod integrity;
pub(crate) use integrity::IntegrityAlgorithm;

mod open_session;

#[derive(Debug, Clone)]
pub struct Message {}

impl Message {
    fn write_data(&self, buffer: &mut Vec<u8>) {
        /*
        let wire = WirePayloadType {
            authenticated: false,
            encrypted: false,
            payload_type: *payload_type,
        };

        wire.write_to(buffer);
        buffer.extend_from_slice(&ssession_id.to_le_bytes());
        buffer.extend_from_slice(&session_sequence_number.to_le_bytes());

        buffer.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        buffer.extend_from_slice(&payload);

        // Pad length
        buffer.push(0);

        // Next header
        buffer.push(0);

        Ok(())

        */

        todo!()
    }

    fn ipmi_v2_0_from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 10 {
            return Err("Not enough data");
        }

        debug_assert!(data[0] == 0x06);

        let (
            WirePayloadType {
                authenticated,
                encrypted,
                payload_type,
            },
            data,
        ) = WirePayloadType::from_data(data).ok_or("Invalid wire payload type.")?;

        let session_id = u32::from_le_bytes(data[..4].try_into().unwrap());
        let session_sequence_number = u32::from_le_bytes(data[4..8].try_into().unwrap());

        let data_len = u16::from_le_bytes(data[8..10].try_into().unwrap());
        let data = &data[10..];

        let payload = if data_len == 0 && data.is_empty() {
            Vec::new()
        } else if data.len() == data_len as usize {
            data.to_vec()
        } else {
            return Err("Payload len is not correct");
        };

        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PayloadType {
    IpmiMessage,
    Sol,
    OemExplicit(u32, u16),
    RmcpPlusOpenSessionRequest,
    RmcpPlusOpenSessionResponse,
    RakpMessage1,
    RakpMessage2,
    RakpMessage3,
    RakpMessage4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct WirePayloadType {
    pub authenticated: bool,
    pub encrypted: bool,
    pub payload_type: PayloadType,
}

impl WirePayloadType {
    // TODO: useful error type
    pub fn from_data(data: &[u8]) -> Option<(Self, &[u8])> {
        assert!(data.len() > 0);

        let authenticated = data[0] & 0x80 == 0x80;
        let encrypted = data[0] & 0x40 == 0x40;
        let ty = data[0] & 0x3F;

        let ty = match ty {
            0x00 => PayloadType::IpmiMessage,
            0x01 => PayloadType::Sol,
            0x02 => {
                if data.len() < 7 {
                    return None;
                }

                let mut oem_iana = [0u8; 4];
                oem_iana[..3].copy_from_slice(&data[1..4]);

                // 4th byte of OEM IANA is reserved
                debug_assert_eq!(data[4], 0);

                let oem_iana = u32::from_le_bytes(oem_iana);
                let oem_payload_id = u16::from_le_bytes(data[5..7].try_into().unwrap());

                let wire = Self {
                    payload_type: PayloadType::OemExplicit(oem_iana, oem_payload_id),
                    authenticated,
                    encrypted,
                };

                return Some((wire, &data[7..]));
            }
            0x10 => PayloadType::RmcpPlusOpenSessionRequest,
            0x11 => PayloadType::RmcpPlusOpenSessionResponse,
            0x12 => PayloadType::RakpMessage1,
            0x13 => PayloadType::RakpMessage2,
            0x14 => PayloadType::RakpMessage3,
            0x15 => PayloadType::RakpMessage4,
            _ => return None,
        };

        let wire = Self {
            payload_type: ty,
            authenticated,
            encrypted,
        };

        Some((wire, &data[1..]))
    }

    pub fn write_to(&self, output: &mut Vec<u8>) {
        let authenticated = (self.authenticated as u8) << 7;
        let encrypted = (self.encrypted as u8) << 6;

        let single_byte = match self.payload_type {
            PayloadType::IpmiMessage => 0x0,
            PayloadType::Sol => 0x01,
            PayloadType::OemExplicit(oem_iana, oem_payload_id) => {
                output.push(authenticated | encrypted | 0x02);

                debug_assert!(oem_iana & 0xFF0000 == 0);

                output.extend_from_slice(&(oem_iana & 0xFFFF).to_le_bytes());
                output.extend_from_slice(&oem_payload_id.to_le_bytes());
                return;
            }
            PayloadType::RmcpPlusOpenSessionRequest => 0x10,
            PayloadType::RmcpPlusOpenSessionResponse => 0x11,
            PayloadType::RakpMessage1 => 0x12,
            PayloadType::RakpMessage2 => 0x13,
            PayloadType::RakpMessage3 => 0x14,
            PayloadType::RakpMessage4 => 0x15,
        };

        output.push(authenticated | encrypted | single_byte);
    }
}
