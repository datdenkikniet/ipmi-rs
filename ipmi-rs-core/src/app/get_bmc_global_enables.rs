use crate::connection::{EncodeIpmiCommand, IpmiCommand, NetFn, NotEnoughData};

/// The Get BMC Global Enables command (IPMI 2.0, Table 22-3).
pub struct GetBmcGlobalEnables;

impl EncodeIpmiCommand for GetBmcGlobalEnables {
    const NETFN: NetFn = NetFn::App;
    const CMD: u8 = 0x2f;

    fn request_data_len(&self) -> usize {
        0
    }

    fn write_request_data(&self, data: &mut [u8]) {
        debug_assert!(data.is_empty());
    }
}

impl IpmiCommand for GetBmcGlobalEnables {
    type Output = BmcGlobalEnables;
    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        BmcGlobalEnables::parse(data).ok_or(NotEnoughData)
    }
}

/// The current BMC global enables (IPMI 2.0, Table 22-3).
#[derive(Clone, Debug, PartialEq)]
pub struct BmcGlobalEnables {
    /// Whether the receive message queue interrupt (including KCS communication interrupts) is enabled.
    pub receive_message_queue_interrupt_enabled: bool,
    /// Whether the event message buffer full interrupt is enabled.
    pub event_message_buffer_full_interrupt_enabled: bool,
    /// Whether the event message buffer is enabled.
    pub event_message_buffer_enabled: bool,
    /// Whether system event logging to the SEL is enabled.
    pub system_event_logging_enabled: bool,
    /// OEM-defined enable; generic management software must ignore this bit.
    pub oem_0_enabled: bool,
    /// OEM-defined enable; generic management software must ignore this bit.
    pub oem_1_enabled: bool,
    /// OEM-defined enable; generic management software must ignore this bit.
    pub oem_2_enabled: bool,
}

impl BmcGlobalEnables {
    /// Parse the command-specific response data, excluding the completion code.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let flags = *data.first()?;

        Some(Self {
            receive_message_queue_interrupt_enabled: flags & 0x01 != 0,
            event_message_buffer_full_interrupt_enabled: flags & 0x02 != 0,
            event_message_buffer_enabled: flags & 0x04 != 0,
            system_event_logging_enabled: flags & 0x08 != 0,
            oem_0_enabled: flags & 0x20 != 0,
            oem_1_enabled: flags & 0x40 != 0,
            oem_2_enabled: flags & 0x80 != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::GetBmcGlobalEnables;
    use crate::connection::{EncodeIpmiCommand, IpmiCommand, NetFn, NotEnoughData};

    #[test]
    fn encodes_empty_app_request() {
        let request = GetBmcGlobalEnables.encode_request(&mut []).unwrap();

        assert_eq!(request.netfn(), NetFn::App);
        assert_eq!(request.cmd(), 0x2f);
        assert!(request.data().is_empty());
    }

    #[test]
    fn decodes_each_enable_and_ignores_reserved_bit() {
        for bit in 0..8 {
            let enables = GetBmcGlobalEnables::parse_success_response(&[1u8 << bit]).unwrap();
            let actual = [
                enables.receive_message_queue_interrupt_enabled,
                enables.event_message_buffer_full_interrupt_enabled,
                enables.event_message_buffer_enabled,
                enables.system_event_logging_enabled,
                false,
                enables.oem_0_enabled,
                enables.oem_1_enabled,
                enables.oem_2_enabled,
            ];
            let expected = core::array::from_fn(|index| index == bit && bit != 4);

            assert_eq!(actual, expected, "bit {bit}");
        }
    }

    #[test]
    fn rejects_missing_enable_byte() {
        assert_eq!(
            GetBmcGlobalEnables::parse_success_response(&[]),
            Err(NotEnoughData)
        );
    }
}
