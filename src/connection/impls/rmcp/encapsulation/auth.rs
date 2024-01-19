use crate::app::auth::AuthType;

fn calculate_md5(password: &[u8; 16], session_id: u32, session_seq: u32, data: &[u8]) -> [u8; 16] {
    let mut context = md5::Context::new();
    context.consume(password);
    context.consume(session_id.to_le_bytes());
    context.consume(data);
    context.consume(session_seq.to_le_bytes());
    context.consume(password);
    *context.compute()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalculateAuthCodeError {
    /// A request was made to calculate the auth code for a message authenticated
    /// using a method that requires a password, but no password was provided.
    MissingPassword,
}

pub(crate) trait AuthExt {
    fn calculate(
        &self,
        password: Option<&[u8; 16]>,
        session_id: u32,
        session_seq: u32,
        data: &[u8],
    ) -> Result<Option<[u8; 16]>, CalculateAuthCodeError>;

    fn verify(
        &self,
        digest: [u8; 16],
        password: Option<&[u8; 16]>,
        session_id: u32,
        session_seq: u32,
        data: &[u8],
    ) -> bool;
}

impl AuthExt for AuthType {
    fn calculate(
        &self,
        password: Option<&[u8; 16]>,
        session_id: u32,
        session_seq: u32,
        data: &[u8],
    ) -> Result<Option<[u8; 16]>, CalculateAuthCodeError> {
        match (self, password) {
            (AuthType::None, _) => Ok(None),
            (AuthType::MD2, Some(_)) => todo!(),
            (AuthType::MD5, Some(password)) => {
                Ok(Some(calculate_md5(password, session_id, session_seq, data)))
            }
            (AuthType::Key, Some(password)) => Ok(Some(password.clone())),
            _ => Err(CalculateAuthCodeError::MissingPassword),
        }
    }

    fn verify(
        &self,
        digest: [u8; 16],
        password: Option<&[u8; 16]>,
        session_id: u32,
        session_seq: u32,
        data: &[u8],
    ) -> bool {
        match (self, password) {
            (AuthType::None, _) => true,
            (AuthType::MD2, Some(_)) => todo!(),
            (AuthType::MD5, Some(password)) => {
                let calc_digest = calculate_md5(password, session_id, session_seq, data);

                calc_digest == digest
            }
            (AuthType::Key, Some(password)) => password == &digest,
            _ => false,
        }
    }
}
