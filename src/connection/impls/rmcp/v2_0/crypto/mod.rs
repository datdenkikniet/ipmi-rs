mod authentication;
pub use authentication::AuthenticationAlgorithm;

mod confidentiality;
pub use confidentiality::ConfidentialityAlgorithm;

mod integrity;
pub use integrity::IntegrityAlgorithm;

use super::{
    open_session::OpenSessionResponse,
    rakp_1_2::{RakpMessageOne, RakpMessageTwo},
};

pub trait Algorithm:
    Sized + Default + PartialEq + PartialOrd + Ord + Into<u8> + TryFrom<u8>
{
}

// TODO: override debug to avoid leaking crypto info
#[derive(Debug)]
pub struct CryptoState {
    confidentiality_algorithm: ConfidentialityAlgorithm,
    authentication_algorithm: AuthenticationAlgorithm,
    integrity_algorithm: IntegrityAlgorithm,
    password: Vec<u8>,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            confidentiality_algorithm: ConfidentialityAlgorithm::None,
            authentication_algorithm: AuthenticationAlgorithm::RakpNone,
            integrity_algorithm: IntegrityAlgorithm::None,
            password: Vec::new(),
        }
    }
}

impl CryptoState {
    #[must_use]
    pub fn configured(&self, password: &[u8], response: &OpenSessionResponse) -> Self {
        let mut me = Self::default();
        me.confidentiality_algorithm = response.confidentiality_payload;
        me.authentication_algorithm = response.authentication_payload;
        me.integrity_algorithm = response.integrity_payload;
        me
    }

    pub fn encrypted(&self) -> bool {
        self.confidentiality_algorithm != ConfidentialityAlgorithm::None
    }

    pub fn authenticated(&self) -> bool {
        self.authentication_algorithm != AuthenticationAlgorithm::RakpNone
    }

    pub fn validate(&self, m1: &RakpMessageOne, m2: &RakpMessageTwo) -> bool {
        match self.authentication_algorithm {
            AuthenticationAlgorithm::RakpNone => todo!(),
            AuthenticationAlgorithm::RakpHmacSha1 => self.validate_hmac_sha1(m1, m2),
            AuthenticationAlgorithm::RakpHmacMd5 => todo!(),
            AuthenticationAlgorithm::RakpHmacSha256 => todo!(),
        }
    }

    fn validate_hmac_sha1(&self, m1: &RakpMessageOne, m2: &RakpMessageTwo) -> bool {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        let mut hmac = Hmac::<Sha1>::new_from_slice(&self.password).unwrap();

        hmac.update(&m2.remote_console_session_id.get().to_le_bytes());
        hmac.update(&m1.managed_system_session_id.get().to_le_bytes());
        hmac.update(&m1.remote_console_random_number);
        hmac.update(&m2.managed_system_random_number);
        hmac.update(&m2.managed_system_guid);
        hmac.update(&[
            u8::from(m1.requested_maximum_privilege_level),
            m1.username.len(),
        ]);
        hmac.update(&m1.username);

        let hmac_output = hmac.finalize().into_bytes();
        println!("{:02X?}", hmac_output);
        println!("{:02X?}", m2.key_exchange_auth_code);

        hmac_output.as_slice() == m2.key_exchange_auth_code
    }

    pub fn read_payload(
        &mut self,
        encrypted: bool,
        authenticated: bool,
        data: &[u8],
    ) -> Result<Vec<u8>, &'static str> {
        assert!(!encrypted);
        assert!(!authenticated);

        if data.len() < 2 {
            return Err("Not enough data");
        }

        if self.encrypted() != encrypted {
            return Err("Mismatching encryption state");
        }

        if self.authenticated() != authenticated {
            return Err("Mismatching authentication state");
        }

        let data_len = u16::from_le_bytes(data[..2].try_into().unwrap());
        let data = &data[2..];

        if data_len as usize == data.len() {
            return Ok(data.to_vec());
        } else {
            Err("Incorrect payload length")
        }
    }

    pub fn write_payload(&mut self, data: &[u8], buffer: &mut Vec<u8>) -> Result<(), &'static str> {
        let data_len = data.len();

        if data_len > u16::MAX as usize {
            return Err("Payload is too long.");
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
