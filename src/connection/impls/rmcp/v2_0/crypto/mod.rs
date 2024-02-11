mod authentication;
pub use authentication::AuthenticationAlgorithm;

mod confidentiality;
pub use confidentiality::ConfidentialityAlgorithm;

mod integrity;
pub use integrity::IntegrityAlgorithm;

pub trait Algorithm:
    Sized + Default + PartialEq + PartialOrd + Ord + Into<u8> + TryFrom<u8>
{
}

// TODO: override debug to avoid leaking crypto info
#[derive(Debug)]
pub struct CryptoState {
    pub confidentiality_algorithm: ConfidentialityAlgorithm,
    pub authentication_algorithm: AuthenticationAlgorithm,
    pub integrity_algorithm: IntegrityAlgorithm,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            confidentiality_algorithm: ConfidentialityAlgorithm::None,
            authentication_algorithm: AuthenticationAlgorithm::RakpNone,
            integrity_algorithm: IntegrityAlgorithm::None,
        }
    }
}

impl CryptoState {
    pub fn encrypted(&self) -> bool {
        self.confidentiality_algorithm != ConfidentialityAlgorithm::None
    }

    pub fn authenticated(&self) -> bool {
        self.authentication_algorithm != AuthenticationAlgorithm::RakpNone
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
