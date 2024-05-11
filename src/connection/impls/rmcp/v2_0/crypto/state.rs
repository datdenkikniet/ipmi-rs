use crate::connection::rmcp::{
    v2_0::{crypto::sha1::Sha1Hmac, ReadError, WriteError},
    Message, OpenSessionResponse as OSR, RakpMessage1 as RM1, RakpMessage2 as RM2,
};

use super::{keys::Keys, AuthenticationAlgorithm, SubState};

pub struct CryptoState {
    password: Vec<u8>,
    kg: Option<[u8; 20]>,
    state: SubState,
}

impl core::fmt::Debug for CryptoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CryptoState")
            .field("password", &"<redacted>")
            .field("kg", &"<redacted>")
            .field("state", &self.state)
            .finish()
    }
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            password: Vec::new(),
            kg: None,
            state: SubState::empty(),
        }
    }
}

impl CryptoState {
    fn kg(&self) -> &[u8] {
        self.kg
            .as_ref()
            .map(|v| &v[..])
            .unwrap_or(self.password.as_ref())
    }
}

impl CryptoState {
    pub fn new(kg: Option<[u8; 20]>, password: &[u8]) -> Self {
        Self {
            kg,
            password: password.to_vec(),
            state: SubState::empty(),
        }
    }

    pub fn calculate_rakp3_data(&mut self, osr: &OSR, m1: &RM1, m2: &RM2) -> Option<Vec<u8>> {
        match osr.authentication_payload {
            AuthenticationAlgorithm::RakpNone => todo!(),
            AuthenticationAlgorithm::RakpHmacSha1 => self.validate_hmac_sha1(osr, m1, m2),
            AuthenticationAlgorithm::RakpHmacMd5 => todo!(),
            AuthenticationAlgorithm::RakpHmacSha256 => todo!(),
        }
    }

    pub fn verify(
        &self,
        algorithm: AuthenticationAlgorithm,
        remote_console_random_number: &[u8; 16],
        managed_system_session_id: u32,
        managed_system_guid: &[u8; 16],
        integrity_check_value: &[u8],
    ) -> bool {
        match algorithm {
            AuthenticationAlgorithm::RakpNone => integrity_check_value.is_empty(),
            AuthenticationAlgorithm::RakpHmacSha1 => {
                let integrity = &Sha1Hmac::new(&self.state.keys.sik)
                    .feed(remote_console_random_number)
                    .feed(&managed_system_session_id.to_le_bytes())
                    .feed(managed_system_guid)
                    .finalize()[..12];

                integrity_check_value == integrity
            }
            AuthenticationAlgorithm::RakpHmacMd5 => todo!(),
            AuthenticationAlgorithm::RakpHmacSha256 => todo!(),
        }
    }

    fn validate_hmac_sha1(&mut self, osr: &OSR, m1: &RM1, m2: &RM2) -> Option<Vec<u8>> {
        let privilege_level_byte = u8::from(m1.requested_maximum_privilege_level);

        let hmac_output = Sha1Hmac::new(&self.password)
            .feed(&m2.remote_console_session_id.get().to_le_bytes())
            .feed(&m1.managed_system_session_id.get().to_le_bytes())
            .feed(&m1.remote_console_random_number)
            .feed(&m2.managed_system_random_number)
            .feed(&m2.managed_system_guid)
            .feed(&[privilege_level_byte, m1.username.len()])
            .feed(m1.username)
            .finalize();

        if hmac_output == m2.key_exchange_auth_code {
            let sik = Sha1Hmac::new(self.kg())
                .feed(&m1.remote_console_random_number)
                .feed(&m2.managed_system_random_number)
                .feed(&[privilege_level_byte, m1.username.len()])
                .feed(m1.username)
                .finalize();

            let output = Sha1Hmac::new(&self.password)
                .feed(&m2.managed_system_random_number)
                .feed(&m2.remote_console_session_id.get().to_le_bytes())
                .feed(&[privilege_level_byte, m1.username.len()])
                .feed(m1.username)
                .finalize();

            let new_state = SubState {
                keys: Keys::from_sik(sik),
                confidentiality_algorithm: osr.confidentiality_payload,
                integrity_algorithm: osr.integrity_payload,
            };

            self.state = new_state;

            Some(output.to_vec())
        } else {
            None
        }
    }
}

impl CryptoState {
    pub fn read_payload(&mut self, data: &mut [u8]) -> Result<Message, ReadError> {
        self.state.read_payload(data)
    }

    pub fn write_unencrypted(message: &Message, buffer: &mut Vec<u8>) -> Result<(), WriteError> {
        SubState::empty().write_payload(message, buffer)
    }

    pub fn write_message(
        &mut self,
        message: &Message,
        buffer: &mut Vec<u8>,
    ) -> Result<(), WriteError> {
        self.state.write_payload(message, buffer)
    }
}
