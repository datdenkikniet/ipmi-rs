use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use ipmi_rs_core::app::auth::{ConfidentialityAlgorithm, IntegrityAlgorithm};

use crate::rmcp::{v2_0::crypto::sha1::Sha1Hmac, Message, PayloadType};

use super::{
    super::{ReadError, WriteError},
    keys::Keys,
    CryptoUnwrapError,
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
            keys: Keys::from_sik([0u8; 20]),
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
                    let integrity_data =
                        Sha1Hmac::new(&self.keys.k1).feed(auth_code_data).finalize();

                    buffer.extend_from_slice(&integrity_data[..12]);
                }
                IntegrityAlgorithm::HmacMd5_128 => todo!(),
                IntegrityAlgorithm::Md5_128 => todo!(),
                IntegrityAlgorithm::HmacSha256_128 => todo!(),
            };
        }

        Ok(())
    }

    fn validate_trailer<'a>(&self, data: &'a mut [u8]) -> Result<&'a mut [u8], CryptoUnwrapError> {
        match self.integrity_algorithm {
            IntegrityAlgorithm::None => Ok(data),
            IntegrityAlgorithm::HmacSha1_96 => {
                let (data, checksum_data) = data.split_at_mut(data.len() - 12);

                let checksum = Sha1Hmac::new(&self.keys.k1).feed(data).finalize();

                if &checksum[..12] != checksum_data {
                    return Err(CryptoUnwrapError::AuthCodeMismatch);
                }

                let data_len = data.len();
                let pad_len = data[data_len - 2] as usize;
                let next_header = data[data_len - 1];

                if next_header != 0x07 {
                    return Err(CryptoUnwrapError::UnknownNextHeader(next_header));
                }

                // strip 2 bytes (pad_len and next_header) and the length
                // of the pad.
                Ok(&mut data[..data_len - 2 - pad_len])
            }
            IntegrityAlgorithm::HmacMd5_128 => todo!(),
            IntegrityAlgorithm::Md5_128 => todo!(),
            IntegrityAlgorithm::HmacSha256_128 => todo!(),
        }
    }

    /// Write payload data `data` to `buffer`, potentially encrypting and adding
    /// headers or trailers as necessary.
    fn write_data_encrypted(
        &mut self,
        data: &[u8],
        buffer: &mut Vec<u8>,
    ) -> Result<(), WriteError> {
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

                let encryptor = cbc::Encryptor::<aes::Aes128>::new(self.keys.aes_key(), &iv.into());

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

    /// Read the (potentially encrypted) payload data from `data`, and return
    /// a buffer containing the decrypted data.
    fn read_data_encrypted<'a>(
        &self,
        data: &'a mut [u8],
    ) -> Result<&'a mut [u8], CryptoUnwrapError> {
        let (data, trailer) = match self.confidentiality_algorithm {
            ConfidentialityAlgorithm::None => {
                const EMPTY_TRAILER: &[u8] = &[];
                (data, EMPTY_TRAILER)
            }
            ConfidentialityAlgorithm::AesCbc128 => {
                let (iv, data_and_trailer) = data.split_at_mut(16);
                let iv: [u8; 16] = iv.try_into().unwrap();

                let decryptor: cbc::Decryptor<aes::Aes128> =
                    cbc::Decryptor::<aes::Aes128>::new(self.keys.aes_key(), &iv.into());

                decryptor
                    .decrypt_padded_mut::<NoPadding>(data_and_trailer)
                    .unwrap();

                let trailer_len = data_and_trailer[data_and_trailer.len() - 1] as usize;
                let data_len = data_and_trailer.len() - trailer_len - 1;

                let (data, trailer) = data_and_trailer.split_at_mut(data_len);

                let trailer = &trailer[..trailer.len() - 1];
                let trailer_len_desc = trailer[trailer.len() - 1] as usize;

                if trailer.len() != trailer_len
                    || trailer.len() != trailer_len_desc
                    || trailer_len != trailer_len_desc
                {
                    return Err(CryptoUnwrapError::IncorrectConfidentialityTrailerLen);
                }

                (data, trailer)
            }
            ConfidentialityAlgorithm::Xrc4_128 => todo!(),
            ConfidentialityAlgorithm::Xrc4_40 => todo!(),
        };

        if trailer.iter().zip(1..).any(|(l, r)| *l != r) {
            Err(CryptoUnwrapError::InvalidConfidentialityTrailer)
        } else {
            Ok(data)
        }
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

        if self.encrypted() != encrypted {
            return Err(CryptoUnwrapError::MismatchingEncryptionState.into());
        }

        if self.authenticated() != authenticated {
            return Err(CryptoUnwrapError::MismatchingAuthenticationState.into());
        }

        let session_id = u32::from_le_bytes(data[2..6].try_into().unwrap());
        let session_sequence_number = u32::from_le_bytes(data[6..10].try_into().unwrap());

        let data_with_header = self.validate_trailer(data)?;
        let data = &mut data_with_header[10..];

        if data.len() < 2 {
            return Err(CryptoUnwrapError::NotEnoughData.into());
        }

        let data_len = u16::from_le_bytes(data[..2].try_into().unwrap());
        let data = &mut data[2..];

        if data_len as usize != data.len() {
            return Err(CryptoUnwrapError::IncorrectPayloadLen.into());
        }

        let data = self.read_data_encrypted(data)?;

        Ok(Message {
            ty,
            session_id,
            session_sequence_number,
            payload: data.to_vec(),
        })
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

        self.write_data_encrypted(&message.payload, buffer)?;
        self.write_trailer(buffer)?;

        Ok(())
    }
}
