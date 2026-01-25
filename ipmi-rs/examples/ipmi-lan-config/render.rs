use crate::types::{ChannelConfig, Ipv6AddressEntry};

pub fn render_schema() -> &'static str {
    include_str!("lan-config-schema.json")
}

pub fn render_ipv6_example() -> &'static str {
    include_str!("lan-config-ipv6-example.json")
}

pub fn render_json(channels: &[ChannelConfig]) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"channels\": [\n");

    for (index, channel) in channels.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }

        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"channel_number\": {},\n",
            channel.channel_number
        ));
        out.push_str("      \"lan_config\": {\n");

        let fields = [
            ("ip_address", &channel.lan_config.ip_address),
            ("subnet_mask", &channel.lan_config.subnet_mask),
            ("gateway", &channel.lan_config.gateway),
            ("mac_address", &channel.lan_config.mac_address),
            ("ip_source", &channel.lan_config.ip_source),
            (
                "default_gateway_mac",
                &channel.lan_config.default_gateway_mac,
            ),
            ("backup_gateway", &channel.lan_config.backup_gateway),
            ("backup_gateway_mac", &channel.lan_config.backup_gateway_mac),
            ("ipv6_ipv4_support", &channel.lan_config.ipv6_ipv4_support),
            (
                "ipv6_ipv4_addressing_enables",
                &channel.lan_config.ipv6_ipv4_addressing_enables,
            ),
            (
                "ipv6_header_static_traffic_class",
                &channel.lan_config.ipv6_header_static_traffic_class,
            ),
            (
                "ipv6_header_static_hop_limit",
                &channel.lan_config.ipv6_header_static_hop_limit,
            ),
            (
                "ipv6_header_flow_label",
                &channel.lan_config.ipv6_header_flow_label,
            ),
            ("ipv6_status", &channel.lan_config.ipv6_status),
        ];

        for (field_index, (name, value)) in fields.iter().enumerate() {
            if field_index > 0 {
                out.push_str(",\n");
            }
            out.push_str(&format!(
                "        \"{}\": {}",
                name,
                json_value(value.as_deref())
            ));
        }

        if let Some(entries) = channel.lan_config.ipv6_static_addresses.as_ref() {
            out.push_str(",\n        \"ipv6_static_addresses\": ");
            out.push_str(&render_ipv6_entries_pretty(entries));
        }

        if let Some(entries) = channel.lan_config.ipv6_dynamic_addresses.as_ref() {
            out.push_str(",\n        \"ipv6_dynamic_addresses\": ");
            out.push_str(&render_ipv6_entries_pretty(entries));
        }

        out.push_str("\n      }\n    }");
    }

    out.push_str("\n  ]\n}\n");
    out
}

fn json_value(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json(value)),
        None => "null".to_string(),
    }
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            _ => ch.to_string(),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_ipv6_entries_pretty(entries: &[Ipv6AddressEntry]) -> String {
    let mut out = String::from("[\n");
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("          { ");
        out.push_str(&format!(
            "\"set_selector\": {}, \"source_type\": {}, \"address\": \"{}\", \"prefix_length\": {}, \"status\": {}, \"status_label\": \"{}\"",
            entry.set_selector,
            entry.source_type,
            escape_json(&entry.address),
            entry.prefix_length,
            entry.status,
            escape_json(&entry.status_label)
        ));
        if let Some(enabled) = entry.enabled {
            out.push_str(&format!(", \"enabled\": {}", enabled));
        }
        out.push_str(" }");
    }
    out.push_str("\n        ]");
    out
}
