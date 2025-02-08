pub fn calculate_md5(
    password: &[u8; 16],
    session_id: u32,
    session_seq: u32,
    data: &[u8],
) -> [u8; 16] {
    let mut context = md5::Context::new();
    context.consume(password);
    context.consume(session_id.to_le_bytes());
    context.consume(data);
    context.consume(session_seq.to_le_bytes());
    context.consume(password);
    *context.compute()
}
