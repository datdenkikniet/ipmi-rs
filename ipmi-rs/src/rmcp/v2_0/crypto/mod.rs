mod sha1;

mod keys;

mod state;
pub use state::CryptoState;

mod sub_state;
pub(crate) use sub_state::SubState;

#[derive(Debug, Clone, PartialEq)]
pub enum CryptoUnwrapError {
    NotEnoughData,
    MismatchingEncryptionState,
    MismatchingAuthenticationState,
    IncorrectPayloadLen,
    IncorrectConfidentialityTrailerLen,
    InvalidConfidentialityTrailer,
    AuthCodeMismatch,
    UnknownNextHeader(u8),
}
