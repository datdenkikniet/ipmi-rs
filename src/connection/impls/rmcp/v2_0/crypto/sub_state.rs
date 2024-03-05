use crate::connection::rmcp::{v2_0::crypto::sha1::RunningHmac, Message, PayloadType};

use super::{
    super::{ReadError, WriteError},
    keys::Keys,
    ConfidentialityAlgorithm, CryptoUnwrapError, IntegrityAlgorithm,
};

pub struct SubState {
    pub(crate) keys: Keys,
    pub(crate) confidentiality_algorithm: ConfidentialityAlgorithm,
    pub(crate) integrity_algorithm: IntegrityAlgorithm,
}

impl core::fmt::Debug for SubState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Configured")
            .field("keys", &self.keys)
            .field("confidentiality_algorithm", &self.confidentiality_algorithm)
            .field("integrity_algorithm", &self.integrity_algorithm)
            .finish()
    }
}

impl SubState {
    pub fn empty() -> Self {
        Self {
            keys: Keys::default(),
            confidentiality_algorithm: ConfidentialityAlgorithm::None,
            integrity_algorithm: IntegrityAlgorithm::None,
        }
    }

    fn encrypted(&self) -> bool {
        self.confidentiality_algorithm != ConfidentialityAlgorithm::None
    }

    fn authenticated(&self) -> bool {
        self.integrity_algorithm != IntegrityAlgorithm::None
    }

    pub fn read_payload(&mut self, data: &[u8]) -> Result<Message, ReadError> {
        if data.len() < 10 {
            return Err(ReadError::NotEnoughData);
        }

        if data[0] != 0x06 {
            return Err(ReadError::NotIpmiV2_0);
        }

        let encrypted = (data[1] & 0x80) == 0x80;
        let authenticated = (data[1] & 0x40) == 0x40;
        let ty = PayloadType::try_from(data[1] & 0x3F)
            .map_err(|_| ReadError::InvalidPayloadType(data[1] & 0x3F))?;

        let session_id = u32::from_le_bytes(data[2..6].try_into().unwrap());
        let session_sequence_number = u32::from_le_bytes(data[6..10].try_into().unwrap());

        let data = &data[10..];

        if data.len() < 2 {
            return Err(CryptoUnwrapError::NotEnoughData.into());
        }

        if self.encrypted() != encrypted {
            return Err(CryptoUnwrapError::MismatchingEncryptionState.into());
        }

        if self.authenticated() != authenticated {
            return Err(CryptoUnwrapError::MismatchingAuthenticationState.into());
        }

        let data_len = u16::from_le_bytes(data[..2].try_into().unwrap());
        let data = &data[2..];

        let data = match self.integrity_algorithm {
            IntegrityAlgorithm::None => data,
            IntegrityAlgorithm::HmacSha1_96 => {
                // TODO: validate
                let pad_len = data[data.len() - 12 - 2];
                &data[..data.len() - 12 - 2 - pad_len as usize]
            }
            IntegrityAlgorithm::HmacMd5_128 => todo!(),
            IntegrityAlgorithm::Md5_128 => todo!(),
            IntegrityAlgorithm::HmacSha256_128 => todo!(),
        };

        if data_len as usize == data.len() {
            // Strip off PAD byte when the message is not out-of-session
            let data = if session_id != 0 && session_sequence_number != 0 {
                &data[..data.len() - 1]
            } else {
                data
            };

            Ok(Message {
                ty,
                session_id,
                session_sequence_number,
                payload: data.to_vec(),
            })
        } else {
            Err(CryptoUnwrapError::IncorrectPayloadLen.into())
        }
    }

    pub fn write_payload(
        &mut self,
        message: &Message,
        buffer: &mut Vec<u8>,
    ) -> Result<(), WriteError> {
        assert_eq!(buffer.len(), 4, "Buffer must only contain RMCP header.");

        buffer.push(0x06);

        let encrypted = (self.encrypted() as u8) << 7;
        let authenticated = (self.authenticated() as u8) << 6;
        buffer.push(encrypted | authenticated | u8::from(message.ty));

        // TODO: support OEM IANA and OEM payload ID? Ignore for now: unsupported payload type

        buffer.extend_from_slice(&message.session_id.to_le_bytes());
        buffer.extend_from_slice(&message.session_sequence_number.to_le_bytes());

        let data = &message.payload;

        let data_len = data.len();

        if data_len > u16::MAX as usize {
            return Err(WriteError::PayloadTooLong);
        }

        // Confidentiality header
        match self.confidentiality_algorithm {
            ConfidentialityAlgorithm::None => {}
            ConfidentialityAlgorithm::AesCbc128 => todo!(),
            ConfidentialityAlgorithm::Xrc4_128 => todo!(),
            ConfidentialityAlgorithm::Xrc4_40 => todo!(),
        }

        // Length
        buffer.extend_from_slice(&(data_len as u16).to_le_bytes());

        // Data
        buffer.extend(data);

        // Confidentiality trailer
        match self.confidentiality_algorithm {
            ConfidentialityAlgorithm::None => {}
            ConfidentialityAlgorithm::AesCbc128 => todo!(),
            ConfidentialityAlgorithm::Xrc4_128 => todo!(),
            ConfidentialityAlgorithm::Xrc4_40 => todo!(),
        }

        // IPMI Session Trailer is only present if packets are authenticated.
        if self.authenticated() {
            // + 2 because pad data and pad length are also covered by
            // integrity checksum.
            let auth_code_data_len = buffer[4..].len() + 2;

            // Integrity PAD
            let pad_length = (4 - auth_code_data_len % 4) % 4;

            buffer.extend(std::iter::repeat(0xFF).take(pad_length));

            // Pad length
            buffer.push(pad_length as u8);

            // Next header
            buffer.push(0x07);

            // AuthCode
            let auth_code_data = &buffer[4..];

            match self.integrity_algorithm {
                IntegrityAlgorithm::None => {}
                IntegrityAlgorithm::HmacSha1_96 => {
                    let integrity_data = RunningHmac::new(&self.keys.k1)
                        .feed(auth_code_data)
                        .finalize();

                    buffer.extend_from_slice(&integrity_data[..12]);
                }
                IntegrityAlgorithm::HmacMd5_128 => todo!(),
                IntegrityAlgorithm::Md5_128 => todo!(),
                IntegrityAlgorithm::HmacSha256_128 => todo!(),
            };
        }

        Ok(())
    }
}
