macro_rules! netfn {
    ($($name:ident => [$req:literal | $resp:literal]),*) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[allow(missing_docs)]
        pub enum NetFn {
            $($name,)*
            /// A reserved netfn.
            Reserved(u8),
        }

        impl From<u8> for NetFn {
            fn from(value: u8) -> Self {
                match value {
                    $($req | $resp => Self::$name,)*
                    v => Self::Reserved(v),
                }
            }
        }

        impl NetFn {
            /// Get the raw data for the request value of this netfn.
            pub fn request_value(&self) -> u8 {
                match self {
                    $(Self::$name => $req,)*
                    Self::Reserved(v) => {
                        if v % 2 == 1 {
                            v - 1
                        } else {
                            *v
                        }
                    }
                }
            }

            /// Get the raw data for the response value of this netfn.
            pub fn response_value(&self) -> u8 {
                match self {
                    $(Self::$name => $resp,)*
                    NetFn::Reserved(v) => {
                        if v % 2 == 0 {
                            v + 1
                        } else {
                            *v
                        }
                    }
                }
            }
        }
    };
}

netfn!(
    Chassis => [0x00 | 0x01],
    Bridge => [0x02 | 0x03],
    SensorEvent => [0x04 | 0x05],
    App => [0x06 | 0x07],
    Firmware => [0x08 | 0x09],
    Storage => [0x0A | 0x0B],
    Transport => [0x0C | 0x0D]
);

impl NetFn {
    /// Check whether `v` is a response value.
    pub fn is_response_value(v: u8) -> bool {
        v % 2 == 0
    }

    /// Check whether `v` is a request value.
    pub fn is_request_value(v: u8) -> bool {
        !Self::is_response_value(v)
    }
}
