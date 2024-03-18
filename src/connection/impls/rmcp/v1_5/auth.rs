use crate::app::auth::AuthType;

use super::{md2::md2, WriteError};

fn calculate_md5(password: &[u8; 16], session_id: u32, session_seq: u32, data: &[u8]) -> [u8; 16] {
    let mut context = md5::Context::new();
    context.consume(password);
    context.consume(session_id.to_le_bytes());
    context.consume(data);
    context.consume(session_seq.to_le_bytes());
    context.consume(password);
    *context.compute()
}

fn calculate_md2(password: &[u8; 16], session_id: u32, session_seq: u32, data: &[u8]) -> [u8; 16] {
    let data = password
        .iter()
        .copied()
        .chain(session_id.to_le_bytes())
        .chain(data.iter().copied())
        .chain(session_seq.to_le_bytes())
        .chain(password.iter().copied());

    md2(data)
}

pub fn calculate(
    ty: &AuthType,
    password: Option<&[u8; 16]>,
    session_id: u32,
    session_seq: u32,
    data: &[u8],
) -> Result<Option<[u8; 16]>, WriteError> {
    match (ty, password) {
        (AuthType::None, _) => Ok(None),
        (AuthType::MD2, Some(password)) => {
            Ok(Some(calculate_md2(password, session_id, session_seq, data)))
        }
        (AuthType::MD5, Some(password)) => {
            Ok(Some(calculate_md5(password, session_id, session_seq, data)))
        }
        (AuthType::Key, Some(password)) => Ok(Some(*password)),
        _ => Err(WriteError::MissingPassword),
    }
}

pub fn verify(
    ty: &AuthType,
    digest: [u8; 16],
    password: Option<&[u8; 16]>,
    session_id: u32,
    session_seq: u32,
    data: &[u8],
) -> bool {
    match (ty, password) {
        (AuthType::None, _) => true,
        (AuthType::MD2, Some(password)) => {
            let calc_digest = calculate_md2(password, session_id, session_seq, data);

            calc_digest == digest
        }
        (AuthType::MD5, Some(password)) => {
            let calc_digest = calculate_md5(password, session_id, session_seq, data);
            calc_digest == digest
        }
        (AuthType::Key, Some(password)) => password == &digest,
        _ => false,
    }
}
