mod authentication;
pub use authentication::AuthenticationAlgorithm;

mod confidentiality;
pub use confidentiality::ConfidentialityAlgorithm;

mod integrity;
pub use integrity::IntegrityAlgorithm;

use super::{messages::OpenSessionResponse, RakpMessage1, RakpMessage2, WriteError};

pub trait Algorithm:
    Sized + Default + PartialEq + PartialOrd + Ord + Into<u8> + TryFrom<u8>
{
}

#[derive(Debug, Clone, PartialEq)]
pub enum CryptoUnwrapError {
    NotEnoughData,
    MismatchingEncryptionState,
    MismatchingAuthenticationState,
    IncorrectPayloadLen,
}

// TODO: override debug to avoid leaking crypto info
#[derive(Debug)]
pub struct CryptoState {
    confidentiality_algorithm: ConfidentialityAlgorithm,
    authentication_algorithm: AuthenticationAlgorithm,
    integrity_algorithm: IntegrityAlgorithm,
    kg: Option<[u8; 20]>,
    password: Vec<u8>,
    sik: Option<[u8; 20]>,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            kg: None,
            confidentiality_algorithm: ConfidentialityAlgorithm::None,
            authentication_algorithm: AuthenticationAlgorithm::RakpNone,
            integrity_algorithm: IntegrityAlgorithm::None,
            password: Vec::new(),
            sik: None,
        }
    }
}

impl CryptoState {
    pub fn new(kg: Option<[u8; 20]>, password: &[u8], response: &OpenSessionResponse) -> Self {
        Self {
            kg,
            confidentiality_algorithm: response.confidentiality_payload,
            authentication_algorithm: response.authentication_payload,
            integrity_algorithm: response.integrity_payload,
            password: password.to_vec(),
            sik: None,
        }
    }

    pub fn encrypted(&self) -> bool {
        self.confidentiality_algorithm != ConfidentialityAlgorithm::None
    }

    pub fn authenticated(&self) -> bool {
        self.authentication_algorithm != AuthenticationAlgorithm::RakpNone
    }

    pub fn validate(&mut self, m1: &RakpMessage1, m2: &RakpMessage2) -> Option<Vec<u8>> {
        match self.authentication_algorithm {
            AuthenticationAlgorithm::RakpNone => todo!(),
            AuthenticationAlgorithm::RakpHmacSha1 => self.validate_hmac_sha1(m1, m2),
            AuthenticationAlgorithm::RakpHmacMd5 => todo!(),
            AuthenticationAlgorithm::RakpHmacSha256 => todo!(),
        }
    }

    fn kg(&self) -> &[u8] {
        self.kg
            .as_ref()
            .map(|v| &v[..])
            .unwrap_or(self.password.as_ref())
    }

    fn validate_hmac_sha1(&mut self, m1: &RakpMessage1, m2: &RakpMessage2) -> Option<Vec<u8>> {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        type HmacSha1 = Hmac<Sha1>;

        let mut hmac = HmacSha1::new_from_slice(&self.password)
            .expect("SHA1 HMAC initialization from bytes is infallible");

        let privilege_level_byte = u8::from(m1.requested_maximum_privilege_level);

        hmac.update(&m2.remote_console_session_id.get().to_le_bytes());
        hmac.update(&m1.managed_system_session_id.get().to_le_bytes());
        hmac.update(&m1.remote_console_random_number);
        hmac.update(&m2.managed_system_random_number);
        hmac.update(&m2.managed_system_guid);
        hmac.update(&[privilege_level_byte, m1.username.len()]);
        hmac.update(&m1.username);

        let hmac_output = hmac.finalize().into_bytes();

        if hmac_output.as_slice() == m2.key_exchange_auth_code {
            let mut hmac = HmacSha1::new_from_slice(self.kg())
                .expect("SHA1 HMAC initialization from bytes is infallible");

            hmac.update(&m1.remote_console_random_number);
            hmac.update(&m2.managed_system_random_number);
            hmac.update(&[privilege_level_byte, m1.username.len()]);
            hmac.update(&m1.username);

            let sik = Some(hmac.finalize().into_bytes().try_into().unwrap());
            self.sik = sik;

            let mut hmac = HmacSha1::new_from_slice(&self.password)
                .expect("SHA1 HMAC initialization from bytes is infallible");

            hmac.update(&m2.managed_system_random_number);
            hmac.update(&m2.remote_console_session_id.get().to_le_bytes());
            hmac.update(&[privilege_level_byte, m1.username.len()]);
            hmac.update(&m1.username);

            Some(hmac.finalize().into_bytes().to_vec())
        } else {
            None
        }
    }

    pub fn read_payload(
        &mut self,
        encrypted: bool,
        authenticated: bool,
        data: &[u8],
    ) -> Result<Vec<u8>, CryptoUnwrapError> {
        assert!(!encrypted);
        assert!(!authenticated);

        if data.len() < 2 {
            return Err(CryptoUnwrapError::NotEnoughData);
        }

        if self.encrypted() != encrypted {
            return Err(CryptoUnwrapError::MismatchingEncryptionState);
        }

        if self.authenticated() != authenticated {
            return Err(CryptoUnwrapError::MismatchingAuthenticationState);
        }

        let data_len = u16::from_le_bytes(data[..2].try_into().unwrap());
        let data = &data[2..];

        if data_len as usize == data.len() {
            return Ok(data.to_vec());
        } else {
            Err(CryptoUnwrapError::IncorrectPayloadLen)
        }
    }

    pub fn write_payload(&mut self, data: &[u8], buffer: &mut Vec<u8>) -> Result<(), WriteError> {
        let data_len = data.len();

        if data_len > u16::MAX as usize {
            return Err(WriteError::PayloadTooLong);
        }

        // Confidentiality header

        // Length
        buffer.extend_from_slice(&(data_len as u16).to_le_bytes());

        // Data
        buffer.extend_from_slice(data);

        // Confidentiality trailer

        // Integrity PAD

        // Pad length
        buffer.push(0);

        // Next header
        buffer.push(0x07);

        // AuthCode

        Ok(())
    }
}
