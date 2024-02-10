use crate::app::auth::PrivilegeLevel;

use super::{AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm};

#[derive(Debug, Clone)]
pub struct OpenSessionRequest {
    pub message_tag: u8,
    pub requested_max_privilege: PrivilegeLevel,
    pub remote_console_session_id: u32,
    pub authentication_algorithm: AuthenticationAlgorithm,
    pub integrity_algorithm: IntegrityAlgorithm,
    pub confidentiality_algorithm: ConfidentialityAlgorithm,
}
