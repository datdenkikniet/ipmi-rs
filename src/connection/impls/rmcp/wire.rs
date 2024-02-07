use std::{
    io::{Error, ErrorKind},
    iter::FusedIterator,
    net::UdpSocket,
    num::NonZeroU32,
};

use crate::{
    app::auth,
    connection::{
        rmcp::{
            encapsulation::IpmiSessionMessage,
            protocol::{RmcpClass, RmcpMessage},
        },
        LogicalUnit, Message, Request, Response,
    },
};

use super::{RmcpError, RmcpUnwrapError};

pub fn checksum(data: impl IntoIterator<Item = u8>) -> impl Iterator<Item = u8> + FusedIterator {
    struct ChecksumIterator<I> {
        checksum: u8,
        yielded_checksum: bool,
        inner: I,
    }

    impl<I: Iterator<Item = u8>> Iterator for ChecksumIterator<I> {
        type Item = u8;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(value) = self.inner.next() {
                self.checksum = self.checksum.wrapping_add(value);
                Some(value)
            } else if !self.yielded_checksum {
                self.yielded_checksum = true;
                self.checksum = !self.checksum;
                self.checksum = self.checksum.wrapping_add(1);
                Some(self.checksum)
            } else {
                None
            }
        }
    }

    impl<I: Iterator<Item = u8>> FusedIterator for ChecksumIterator<I> {}

    ChecksumIterator {
        checksum: 0,
        yielded_checksum: false,
        inner: data.into_iter(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn send_v1_5(
    inner: &mut UdpSocket,
    auth_type: auth::AuthType,
    requestor_addr: u8,
    responder_addr: u8,
    ipmb_sequence: &mut u8,
    requestor_lun: LogicalUnit,
    request_sequence: &mut u32,
    session_id: Option<NonZeroU32>,
    password: Option<&[u8; 16]>,
    request: &mut Request,
) -> Result<usize, RmcpError> {
    log::trace!("Sending message with auth type {:?}", auth_type);

    let rs_addr = responder_addr;
    let netfn_rslun: u8 = (request.netfn().request_value() << 2) | request.target().lun().value();

    let first_part = checksum([rs_addr, netfn_rslun]);

    let req_addr = requestor_addr;

    let ipmb_sequence_val = *ipmb_sequence;
    *ipmb_sequence = ipmb_sequence.wrapping_add(1);
    let ipmb_sequence = ipmb_sequence_val;

    let reqseq_lun = (ipmb_sequence << 2) | requestor_lun.value();
    let cmd = request.cmd();
    let second_part = checksum(
        [req_addr, reqseq_lun, cmd]
            .into_iter()
            .chain(request.data().iter().copied()),
    );

    let final_data: Vec<_> = first_part.chain(second_part).collect();

    let session_sequence = *request_sequence;

    // Only increment the request sequence once a session has been established
    // succesfully.
    if session_id.is_some() {
        *request_sequence = request_sequence.wrapping_add(1);
    }

    let message = RmcpMessage::new(
        0xFF,
        RmcpClass::Ipmi(IpmiSessionMessage::Ipmiv1_5 {
            auth_type,
            session_sequence_number: session_sequence,
            session_id: session_id.map(|v| v.get()).unwrap_or(0),
            payload: final_data,
        }),
    );

    let send_bytes = message.to_bytes(password)?;

    inner.send(&send_bytes).map_err(Into::into)
}

#[derive(Debug)]
pub enum RmcpReceiveError {
    /// An RMCP error occured.
    Rmcp(RmcpUnwrapError),
    /// The packet did not contain enough data to form a valid RMCP message.
    NotEnoughData,
}

pub fn recv(password: Option<&[u8; 16]>, inner: &mut UdpSocket) -> Result<Response, RmcpError> {
    let mut buffer = [0u8; 1024];
    let received_bytes = inner.recv(&mut buffer)?;

    let data = &buffer[..received_bytes];

    let rcmp_message = RmcpMessage::from_bytes(password, data).map_err(RmcpReceiveError::Rmcp)?;

    let encapsulated_message = if let RmcpMessage {
        class_and_contents: RmcpClass::Ipmi(message),
        ..
    } = rcmp_message
    {
        message
    } else {
        return Err(Error::new(ErrorKind::Other, "RMCP response does not have IPMI class").into());
    };

    let data = match encapsulated_message {
        IpmiSessionMessage::Ipmiv1_5 { payload, .. } => payload,
        IpmiSessionMessage::Ipmiv2_0 { .. } => todo!(),
    };

    if data.len() < 7 {
        return Err(RmcpReceiveError::NotEnoughData.into());
    }

    let _req_addr = data[0];
    let netfn = data[1] >> 2;
    let _checksum1 = data[2];
    let _rs_addr = data[3];
    let _rqseq = data[4];
    let cmd = data[5];
    let response_data: Vec<_> = data[6..data.len() - 1].to_vec();
    let _checksum2 = data[data.len() - 1];

    // TODO: validate sequence, checksums, etc.

    let response = if let Some(resp) = Response::new(Message::new_raw(netfn, cmd, response_data), 0)
    {
        resp
    } else {
        // TODO: need better message here :)
        return Err(Error::new(ErrorKind::Other, "Response data was empty").into());
    };

    Ok(response)
}

#[test]
pub fn checksum_test() {
    let _output: Vec<_> = checksum([0x20, 0x06 << 2]).collect();
}
