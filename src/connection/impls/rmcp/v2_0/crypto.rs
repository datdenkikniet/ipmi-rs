use super::{AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm};

#[derive(Debug)]
pub enum HandshakeState {}

#[derive(Debug)]
pub struct CryptoState {
    pub confidentiality_algorithm: Option<ConfidentialityAlgorithm>,
    pub authentication_algorithm: Option<AuthenticationAlgorithm>,
    pub integrity_algorithm: Option<IntegrityAlgorithm>,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            confidentiality_algorithm: None,
            authentication_algorithm: None,
            integrity_algorithm: None,
        }
    }
}

impl CryptoState {
    pub fn encrypted(&self) -> bool {
        self.confidentiality_algorithm.is_some()
    }

    pub fn authenticated(&self) -> bool {
        self.authentication_algorithm.is_some()
    }

    pub fn read_payload(
        &mut self,
        encrypted: bool,
        authenticated: bool,
        data: &[u8],
    ) -> Result<Vec<u8>, &'static str> {
        assert!(!encrypted);
        assert!(!authenticated);
        assert!(self.confidentiality_algorithm.is_none());
        assert!(self.authentication_algorithm.is_none());
        assert!(self.integrity_algorithm.is_none());

        if data.len() < 2 {
            return Err("Not enough data");
        }

        if self.confidentiality_algorithm.is_some() != encrypted {
            return Err("Mismatching encryption state");
        }

        if self.authentication_algorithm.is_some() != authenticated {
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
        assert!(self.confidentiality_algorithm.is_none());
        assert!(self.authentication_algorithm.is_none());
        assert!(self.integrity_algorithm.is_none());

        let data_len = 2 + data.len();

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
