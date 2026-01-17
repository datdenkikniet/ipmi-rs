use ipmi_rs::transport::{IpAddressSource, Ipv6Address, Ipv6Ipv4Enables};

pub fn parse_ipv4(value: &str) -> Option<ipmi_rs::transport::Ipv4Address> {
    let mut parts = [0u8; 4];
    let mut index = 0;
    for part in value.split('.') {
        if index >= 4 {
            return None;
        }
        let parsed = part.parse::<u8>().ok()?;
        parts[index] = parsed;
        index += 1;
    }
    if index != 4 {
        return None;
    }
    Some(ipmi_rs::transport::Ipv4Address(parts))
}

pub fn parse_mac(value: &str) -> Option<ipmi_rs::transport::MacAddress> {
    let mut parts = [0u8; 6];
    let mut index = 0;
    for part in value.split(':') {
        if index >= 6 {
            return None;
        }
        let parsed = u8::from_str_radix(part, 16).ok()?;
        parts[index] = parsed;
        index += 1;
    }
    if index != 6 {
        return None;
    }
    Some(ipmi_rs::transport::MacAddress(parts))
}

pub fn parse_ip_source(value: &str) -> Option<IpAddressSource> {
    let lower = value.to_ascii_lowercase();
    match lower.as_str() {
        "unspecified" => Some(IpAddressSource::Unspecified),
        "static" => Some(IpAddressSource::Static),
        "dhcp" => Some(IpAddressSource::Dhcp),
        "bios" | "bios/system software" | "bios-system software" => {
            Some(IpAddressSource::BiosOrSystemSoftware)
        }
        "other" => Some(IpAddressSource::Other),
        _ => {
            if let Some(hex) = lower.strip_prefix("0x") {
                u8::from_str_radix(hex, 16)
                    .ok()
                    .map(IpAddressSource::Reserved)
            } else {
                None
            }
        }
    }
}

pub fn parse_ipv6_ipv4_enables(value: &str) -> Option<Ipv6Ipv4Enables> {
    let lower = value.to_ascii_lowercase();
    match lower.as_str() {
        "disabled" | "ipv6 disabled" => Some(Ipv6Ipv4Enables::Ipv6Disabled),
        "ipv6 only" | "ipv6-only" => Some(Ipv6Ipv4Enables::Ipv6Only),
        "dual" | "dual stack" | "ipv6/ipv4" | "ipv6/ipv4 simultaneous" => {
            Some(Ipv6Ipv4Enables::Ipv6Ipv4Simultaneous)
        }
        _ => {
            if let Some(hex) = lower.strip_prefix("0x") {
                u8::from_str_radix(hex, 16)
                    .ok()
                    .map(Ipv6Ipv4Enables::Reserved)
            } else {
                None
            }
        }
    }
}

pub fn parse_u8(value: &str) -> Option<u8> {
    if let Some(hex) = value.strip_prefix("0x") {
        u8::from_str_radix(hex, 16).ok()
    } else {
        value.parse::<u8>().ok()
    }
}

pub fn parse_u24(value: &str) -> Option<Vec<u8>> {
    let raw = if let Some(hex) = value.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        value.parse::<u32>().ok()?
    };
    if raw > 0x000F_FFFF {
        return None;
    }
    let bytes = [
        ((raw >> 16) & 0xFF) as u8,
        ((raw >> 8) & 0xFF) as u8,
        (raw & 0xFF) as u8,
    ];
    Some(bytes.to_vec())
}

pub fn parse_ipv6(value: &str) -> Option<Ipv6Address> {
    let addr = value.parse::<std::net::Ipv6Addr>().ok()?;
    Some(Ipv6Address(addr.octets()))
}
