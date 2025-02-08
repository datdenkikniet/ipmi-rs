#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResponseUnavailableReason {
    Unknown,
    SDRInUpdate,
    DeviceInFwUpdate,
    BMCInitializing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum CompletionErrorCode {
    NodeBusy,
    InvalidCommand,
    InvalidCommandForLun,
    ProcessingTimeout,
    OutOfSpace,
    ReservationCancelledOrInvalidId,
    RequestDataTruncated,
    RequestDataLenInvalid,
    RequestDataLengthLimitExceeded,
    ParameterOutOfRange,
    CannotReturnNumOfRequestedBytes,
    RequestedDatapointNotPresent,
    InvalidDataFieldInRequest,
    CommandIllegalForSensorOrRecord,
    ResponseUnavailable { reason: ResponseUnavailableReason },
    CannotExecuteDuplicateRequest,
    DestinationUnavailable,
    InsufficientPrivilege,
    CannotExecuteCommandInCurrentState,
    SubFunctionDisabled,
    Unspecified,
    Oem(u8),
    CommandSpecific(u8),
    Reserved(u8),
}

impl TryFrom<u8> for CompletionErrorCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value = match value {
            0 => return Err(()),
            0xC0 => Self::NodeBusy,
            0xC1 => Self::InvalidCommand,
            0xC2 => Self::InvalidCommandForLun,
            0xC3 => Self::ProcessingTimeout,
            0xC4 => Self::OutOfSpace,
            0xC5 => Self::ReservationCancelledOrInvalidId,
            0xC6 => Self::RequestDataTruncated,
            0xC7 => Self::RequestDataLenInvalid,
            0xC8 => Self::RequestDataLengthLimitExceeded,
            0xC9 => Self::ParameterOutOfRange,
            0xCA => Self::CannotReturnNumOfRequestedBytes,
            0xCB => Self::RequestedDatapointNotPresent,
            0xCC => Self::InvalidDataFieldInRequest,
            0xCD => Self::CommandIllegalForSensorOrRecord,
            0xCE => Self::ResponseUnavailable {
                reason: ResponseUnavailableReason::Unknown,
            },
            0xCF => Self::CannotExecuteDuplicateRequest,
            0xD0 => Self::ResponseUnavailable {
                reason: ResponseUnavailableReason::SDRInUpdate,
            },
            0xD1 => Self::ResponseUnavailable {
                reason: ResponseUnavailableReason::DeviceInFwUpdate,
            },
            0xD2 => Self::ResponseUnavailable {
                reason: ResponseUnavailableReason::BMCInitializing,
            },
            0xD3 => Self::DestinationUnavailable,
            0xD4 => Self::InsufficientPrivilege,
            0xD5 => Self::CannotExecuteCommandInCurrentState,
            0xD6 => Self::SubFunctionDisabled,
            0xFF => Self::Unspecified,
            0x01..=0x7E => Self::Oem(value),
            0x80..=0xBE => Self::CommandSpecific(value),
            v => Self::Reserved(v),
        };

        Ok(value)
    }
}

impl CompletionErrorCode {
    /// Whether this completion code is a reserved value or not.
    pub fn is_reserved(&self) -> bool {
        matches!(self, Self::Reserved(_))
    }
}
