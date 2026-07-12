pub mod json;

use std::net::IpAddr;

use uuid::Version;

use crate::structs::{rkyv::uuid_version_wrapper::UUIDVersionWrapper, validation::IpVersion};

pub fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn is_valid_email(email: &str) -> bool {
    if email.is_empty() || email.len() > 254 {
        return false;
    }

    let mut parts = email.splitn(2, '@');
    let local_part = parts.next().unwrap_or("");
    let domain_part = parts.next().unwrap_or("");

    if local_part.is_empty() || domain_part.is_empty() {
        return false;
    }

    if local_part.len() > 64 {
        return false;
    }

    if !domain_part.contains('.') || domain_part.starts_with('.') || domain_part.ends_with('.') {
        return false;
    }

    let is_valid_local_char = |c: char| c.is_ascii_alphanumeric() || "._+-".contains(c);
    let is_valid_domain_char = |c: char| c.is_ascii_alphanumeric() || ".-".contains(c);

    local_part.chars().all(is_valid_local_char) && domain_part.chars().all(is_valid_domain_char)
}

pub fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }

    let hex_part = &color[1..];
    let len = hex_part.len();

    // 3 (RGB), 4 (RGBA), 6 (RRGGBB), 8 (RRGGBBAA)
    if len != 3 && len != 4 && len != 6 && len != 8 {
        return false;
    }

    hex_part.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn is_valid_ip(s: &str, version: &Option<IpVersion>) -> bool {
    match s.parse::<IpAddr>() {
        Ok(parsed_addr) => match (version, parsed_addr) {
            (Some(IpVersion::V4), IpAddr::V4(_)) => true,
            (Some(IpVersion::V6), IpAddr::V6(_)) => true,
            (None, _) => true,
            _ => false,
        },

        Err(_) => false,
    }
}

pub fn is_valid_mac_address(mac: &str) -> bool {
    let len = mac.len();

    if len == 17 {
        let separator = mac.chars().nth(2).unwrap();
        if separator != ':' && separator != '-' {
            return false;
        }

        for (i, c) in mac.chars().enumerate() {
            if i == 2 || i == 5 || i == 8 || i == 11 || i == 14 {
                if c != separator {
                    return false;
                }
            } else if !c.is_ascii_hexdigit() {
                return false;
            }
        }
        true
    } else if len == 19 {
        for (i, c) in mac.chars().enumerate() {
            if i == 4 || i == 9 {
                if c != '.' {
                    return false;
                }
            } else if !c.is_ascii_hexdigit() {
                return false;
            }
        }
        true
    } else if len == 12 {
        mac.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        false
    }
}

pub fn is_valid_url(url: &str) -> bool {
    let mut parts = url.splitn(2, "://");

    let scheme = match parts.next() {
        Some(s) if !s.is_empty() => s,
        _ => return false,
    };

    let mut scheme_chars = scheme.chars();
    if !scheme_chars
        .next()
        .map_or(false, |c| c.is_ascii_alphabetic())
    {
        return false;
    }
    if !scheme_chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '.' || c == '-') {
        return false;
    }

    let rest = match parts.next() {
        Some(r) if !r.is_empty() => r,
        _ => return false,
    };

    let host = rest
        .split('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");

    // 5. Host cannot be empty
    !host.is_empty()
}

pub fn is_valid_ulid(ulid: &str) -> bool {
    if ulid.len() != 26 || !ulid.is_ascii() {
        return false;
    }

    let first_char = ulid.as_bytes()[0];
    if !first_char.is_ascii_digit() || first_char > b'7' {
        return false;
    }

    ulid.bytes().all(|b| match b {
        b'0'..=b'9' => true,
        b'A'..=b'H' | b'J'..=b'K' | b'M'..=b'N' | b'P'..=b'T' | b'V'..=b'Z' => true,
        b'a'..=b'h' | b'j'..=b'k' | b'm'..=b'n' | b'p'..=b't' | b'v'..=b'z' => true,
        _ => false,
    })
}

pub fn is_valid_uuid(uuid_str: &str, expected_version: &Option<UUIDVersionWrapper>) -> bool {
    if uuid_str.len() != 36 || !uuid_str.is_ascii() {
        return false;
    }

    let bytes = uuid_str.as_bytes();

    if bytes[8] != b'-' || bytes[13] != b'-' || bytes[18] != b'-' || bytes[23] != b'-' {
        return false;
    }

    let version_byte = bytes[14];

    for (i, &b) in bytes.iter().enumerate() {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            continue;
        }

        if !b.is_ascii_hexdigit() {
            return false;
        }
    }

    if let Some(UUIDVersionWrapper(version)) = expected_version {
        let expected_char = match version {
            Version::Nil => b'0',
            Version::Mac => b'1',
            Version::Dce => b'2',
            Version::Md5 => b'3',
            Version::Random => b'4',
            Version::Sha1 => b'5',
            Version::SortMac => b'6',
            Version::SortRand => b'7',
            Version::Custom => b'8',
            _ => return false,
        };

        if version_byte.to_ascii_lowercase() != expected_char {
            return false;
        }
    }

    true
}
