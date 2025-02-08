use crate::connection::{
    rmcp::{AuthenticationAlgorithm, ConfidentialityAlgorithm, IntegrityAlgorithm},
    Channel, IpmiCommand, Message, NetFn, NotEnoughData,
};

#[derive(Debug, Clone)]
pub struct GetChannelCipherSuites {
    channel: Channel,
    list_index: u8,
}

impl From<GetChannelCipherSuites> for Message {
    fn from(value: GetChannelCipherSuites) -> Self {
        Message::new_request(
            NetFn::App,
            0x54,
            vec![value.channel.value(), 0x00, value.list_index],
        )
    }
}

impl GetChannelCipherSuites {
    /// Create a new `GetChannelCipherSuites`.
    ///
    /// Returns `None` if `list_index > 0x3F``
    pub fn new(channel: Channel, list_index: u8) -> Option<Self> {
        if list_index > 0x3F {
            None
        } else {
            Some(Self {
                channel,
                list_index,
            })
        }
    }
}

pub struct ChannelCipherSuites {
    data_length: usize,
    record_data: [u8; 16],
}

impl ChannelCipherSuites {
    pub fn parse_full_data(data: &[u8]) -> impl Iterator<Item = CipherSuite> + '_ {
        struct Iter<'a> {
            inner: core::slice::Iter<'a, u8>,
            failed: bool,
        }

        impl Iterator for Iter<'_> {
            type Item = CipherSuite;

            fn next(&mut self) -> Option<Self::Item> {
                if self.failed {
                    return None;
                }

                let start_of_record = *self.inner.next()?;

                if start_of_record == 0xC0 {
                    let id = if let Some(id) = self.inner.next() {
                        *id
                    } else {
                        log::warn!("Got correct start of record, but missing Cipher Suite ID.");
                        return None;
                    };

                    let suite = if let Some(suite) = CipherSuite::from_id(id) {
                        suite
                    } else {
                        log::warn!("Unknown cipher suite ID: 0x{id:02X?}");
                        return None;
                    };

                    if let Some(auth) = self.inner.next() {
                        *auth & 0x3F
                    } else {
                        log::warn!("Got correct start of record, but missing Authentication Algorithm Number.");
                        return None;
                    };

                    if let Some(integ) = self.inner.next() {
                        *integ & 0x3F
                    } else {
                        log::warn!(
                            "Got correct start of record, but missing Integrity Algorithm Number."
                        );
                        return None;
                    };

                    if let Some(conf) = self.inner.next() {
                        *conf & 0x3F
                    } else {
                        log::warn!("Got correct start of record, but missing Confidentiality Algorithm Number.");
                        return None;
                    };

                    Some(suite)
                } else {
                    log::debug!("Got a non-standard start of record: {start_of_record:02X}");
                    self.failed = true;
                    None
                }
            }
        }

        Iter {
            inner: data.iter(),
            failed: false,
        }
    }
}

impl core::ops::Deref for ChannelCipherSuites {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.record_data[..self.data_length]
    }
}

impl IpmiCommand for GetChannelCipherSuites {
    type Output = ChannelCipherSuites;

    type Error = NotEnoughData;

    fn parse_success_response(data: &[u8]) -> Result<Self::Output, Self::Error> {
        if data.len() > 16 {
            return Err(NotEnoughData);
        }

        let mut record_data = [0u8; 16];
        record_data[..data.len()].copy_from_slice(data);

        Ok(ChannelCipherSuites {
            record_data,
            data_length: data.len(),
        })
    }
}

macro_rules ! cipher_suite {
    ($([$id:ident, $id_value:literal, $auth:literal, $integrity:literal, $confidentiality:literal]),*) => {

        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum CipherSuite {
            $(
                $id,
            )*
        }

        impl CipherSuite {
            pub fn id(&self) -> u8 {
                match self {
                    $(
                        Self::$id => $id_value,
                    )*
                }
            }

            pub fn from_id(value: u8) -> Option<Self> {
                match value {
                    $(
                        $id_value => Some(Self::$id),
                    )*
                    _ => None,
                }
            }

            pub fn from_suite(suite: [u8; 3]) -> Option<Self> {
                match suite {
                    $(
                        [$auth, $integrity, $confidentiality] => return Some(Self::$id),
                    )*
                    _ => return None,
                }
            }

            pub fn into_suite(&self) -> [u8; 3] {
                match self {
                    $(
                        Self::$id => [$auth, $integrity, $confidentiality],
                    )*
                }
            }

            pub fn as_suite(&self) -> &'static [u8; 3] {
                match self {
                    $(
                        Self::$id => &[$auth, $integrity, $confidentiality],
                    )*
                }
            }

            pub fn authentication(&self) -> AuthenticationAlgorithm {
                let auth = self.as_suite()[0];
                TryFrom::try_from(auth).unwrap()
            }

            pub fn integrity(&self) -> IntegrityAlgorithm {
                let integ = self.as_suite()[1];
                TryFrom::try_from(integ).unwrap()
            }

            pub fn confidentiality(&self) -> ConfidentialityAlgorithm {
                let conf = self.as_suite()[1];
                TryFrom::try_from(conf).unwrap()
            }
        }
    }
}

cipher_suite! {
    [Id0, 0, 0x00, 0x00, 0x00],
    [Id1, 1, 0x01, 0x00, 0x00],
    [Id2, 2, 0x01, 0x01, 0x00],
    [Id3, 3, 0x01, 0x01, 0x01],
    [Id4, 4, 0x01, 0x01, 0x02],
    [Id5, 5, 0x01, 0x01, 0x03],
    [Id6, 6, 0x02, 0x00, 0x00],
    [Id7, 7, 0x02, 0x02, 0x00],
    [Id8, 8, 0x02, 0x02, 0x01],
    [Id9, 9, 0x02, 0x02, 0x02],
    [Id10, 10, 0x02, 0x02, 0x03],
    [Id11, 11, 0x02, 0x03, 0x00],
    [Id12, 12, 0x02, 0x03, 0x01],
    [Id13, 13, 0x02, 0x03, 0x02],
    [Id14, 14, 0x02, 0x03, 0x03],
    [Id15, 15, 0x03, 0x00, 0x00],
    [Id16, 16, 0x03, 0x04, 0x00],
    [Id17, 17, 0x03, 0x04, 0x01],
    [Id18, 18, 0x03, 0x04, 0x02],
    [Id19, 19, 0x03, 0x04, 0x03]
}
