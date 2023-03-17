#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetFn {
    App,
    Storage,
    Unknown(u8),
}

impl From<u8> for NetFn {
    fn from(value: u8) -> Self {
        match value {
            0x06 | 0x07 => Self::App,
            0x0A | 0x0B => Self::Storage,
            v => Self::Unknown(v),
        }
    }
}

impl NetFn {
    pub fn request_value(&self) -> u8 {
        match self {
            NetFn::App => 0x06,
            NetFn::Storage => 0x0A,
            NetFn::Unknown(v) => {
                if v % 2 == 1 {
                    v - 1
                } else {
                    *v
                }
            }
        }
    }

    pub fn response_value(&self) -> u8 {
        match self {
            NetFn::App => 0x07,
            NetFn::Storage => 0x0B,
            NetFn::Unknown(v) => {
                if v % 2 == 0 {
                    v + 1
                } else {
                    *v
                }
            }
        }
    }
}
