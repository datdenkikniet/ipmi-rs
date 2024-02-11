#[derive(Debug, Clone)]
pub struct RakpMessage4<'a> {
    pub message_tag: u8,
    pub status_code: u8,
    pub management_console_session_id: u32,
    pub integrity_check_value: &'a [u8],
}

impl<'a> RakpMessage4<'a> {
    pub fn from_data(data: &'a [u8]) -> Result<Self, &'static str> {
        // 4 = tag, status code, reserved bytes
        if data.len() < 4 {
            return Err("Not enough data");
        }

        let message_tag = data[0];
        let status_code = data[1];

        if status_code != 0 {
            return Err("RMCP+ status code does not indicate success");
        }

        if data.len() < 8 {
            return Err("Not enough data");
        }

        let management_console_session_id = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let integrity_check_value = &data[8..];

        Ok(Self {
            message_tag,
            status_code,
            management_console_session_id,
            integrity_check_value,
        })
    }
}
