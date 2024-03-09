use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

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

    pub fn read_payload(&mut self, data: &mut [u8]) -> Result<Message, ReadError> {
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

        let data = &mut data[10..];

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
        let data = &mut data[2..];

        let data = match self.integrity_algorithm {
            IntegrityAlgorithm::None => data,
            IntegrityAlgorithm::HmacSha1_96 => {
                let data_len = data.len();
                // TODO: validate
                let pad_len = data[data_len - 12 - 2];
                &mut data[..data_len - 12 - 2 - pad_len as usize]
            }
            IntegrityAlgorithm::HmacMd5_128 => todo!(),
            IntegrityAlgorithm::Md5_128 => todo!(),
            IntegrityAlgorithm::HmacSha256_128 => todo!(),
        };

        let data = match self.confidentiality_algorithm {
            ConfidentialityAlgorithm::None => data,
            ConfidentialityAlgorithm::AesCbc128 => {
                let (iv, data_and_trailer) = data.split_at_mut(16);
                let iv: [u8; 16] = iv.try_into().unwrap();

                let decryptor: cbc::Decryptor<aes::Aes128> = cbc::Decryptor::<aes::Aes128>::new(
                    self.keys.k2[..16].try_into().unwrap(),
                    &iv.try_into().unwrap(),
                );

                decryptor
                    .decrypt_padded_mut::<NoPadding>(data_and_trailer)
                    .unwrap();

                let trailer_len = data_and_trailer[data_and_trailer.len() - 1] as usize;
                let data_len = data_and_trailer.len() - trailer_len - 1;

                // TODO: validate trailer

                &mut data_and_trailer[..data_len]
            }
            ConfidentialityAlgorithm::Xrc4_128 => todo!(),
            ConfidentialityAlgorithm::Xrc4_40 => todo!(),
        };

        // TODO: validate data len
        // if data_len as usize == data.len() {
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
        // } else {
        //     Err(CryptoUnwrapError::IncorrectPayloadLen.into())
        // }
    }

    fn write_payload_data(&mut self, data: &[u8], buffer: &mut Vec<u8>) -> Result<(), WriteError> {
        let data_len = data.len();

        if data_len > u16::MAX as usize {
            return Err(WriteError::PayloadTooLong);
        }

        match self.confidentiality_algorithm {
            ConfidentialityAlgorithm::None => {
                // Length
                buffer.extend_from_slice(&(data_len as u16).to_le_bytes());

                // Data
                buffer.extend(data)
            }
            ConfidentialityAlgorithm::AesCbc128 => {
                let mut iv = [0u8; 16];
                getrandom::getrandom(&mut iv).unwrap();

                // Length
                // Data + Confidentiality pad length + header
                let non_pad_len = data_len + 1 + 16;
                let pad_len = (16 - (non_pad_len % 16)) % 16;
                let padded_len = non_pad_len + pad_len;

                if padded_len > u16::MAX as usize {
                    return Err(WriteError::EncryptedPayloadTooLong);
                }

                buffer.extend((padded_len as u16).to_le_bytes());

                // Confidentiality header
                buffer.extend(iv);

                let encryptor = cbc::Encryptor::<aes::Aes128>::new(
                    self.keys.k2[..16].try_into().unwrap(),
                    &iv.try_into().unwrap(),
                );

                let dont_encrypt_len = buffer.len();

                // Data
                buffer.extend(data);

                // Confidentiality trailer
                buffer.extend((1u8..).take(pad_len));
                buffer.push(pad_len as u8);

                let buffer_to_encrypt = &mut buffer[dont_encrypt_len..];

                let encrypted = encryptor
                    .encrypt_padded_mut::<NoPadding>(buffer_to_encrypt, buffer_to_encrypt.len())
                    .unwrap();

                assert_eq!(16 + encrypted.len(), padded_len);
            }
            ConfidentialityAlgorithm::Xrc4_128 => todo!(),
            ConfidentialityAlgorithm::Xrc4_40 => todo!(),
        }

        Ok(())
    }

    fn write_trailer(&mut self, buffer: &mut Vec<u8>) -> Result<(), WriteError> {
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

        self.write_payload_data(&message.payload, buffer)?;

        self.write_trailer(buffer)?;

        Ok(())
    }
}
