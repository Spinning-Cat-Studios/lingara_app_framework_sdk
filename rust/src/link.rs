//! The `link` rule (ADR 30.9.26am D5): `https`, no user information, a host
//! name (never an IP literal), at most `MAX_URL_BYTES`.
//!
//! The relay parses with the WHATWG URL parser. The kit takes no URL crate
//! (D1: its dependencies are the library's), so this reads the authority the
//! way that parser finds it for a special scheme, and where the two could
//! differ it refuses: any `@`, and a `%` in the host, are refused outright
//! rather than decoded.

use crate::limits::MAX_URL_BYTES;

/// Code points the WHATWG parser forbids in a domain, beyond C0 controls,
/// space and DEL.
const FORBIDDEN_IN_HOST: &[char] = &['#', '/', ':', '<', '>', '?', '@', '[', '\\', ']', '^', '|', '%'];

pub(crate) fn is_safe_link(raw: &str) -> bool {
    if raw.len() > MAX_URL_BYTES {
        return false;
    }
    // The parser trims C0 controls and spaces at the ends and drops tabs and
    // newlines anywhere (a newline is refused earlier, as control_chars).
    let url: String = raw.trim_matches(|c: char| c <= ' ').chars().filter(|c| !matches!(c, '\t' | '\n' | '\r')).collect();
    let Some((scheme, rest)) = url.split_once(':') else { return false };
    if !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    let rest = rest.trim_start_matches(['/', '\\']);
    let authority = &rest[..rest.find(['/', '\\', '?', '#']).unwrap_or(rest.len())];
    if authority.contains('@') || authority.starts_with('[') {
        return false;
    }
    let (host, port) = authority.rsplit_once(':').unwrap_or((authority, ""));
    port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u32>().map_or(port.is_empty(), |p| p <= 65_535) && is_domain(host)
}

/// A non-empty name the parser would keep as a domain: no forbidden code
/// point, and not a dotted number, which it would read as IPv4.
fn is_domain(host: &str) -> bool {
    let forbidden = |c: char| c <= ' ' || c == '\u{7F}' || FORBIDDEN_IN_HOST.contains(&c);
    !host.is_empty() && !host.chars().any(forbidden) && !ends_in_a_number(host)
}

/// WHATWG's "ends in a number": the last label (after one trailing dot) is
/// decimal digits or `0x` hex, so the host is an IPv4 address or invalid.
fn ends_in_a_number(host: &str) -> bool {
    let host = host.strip_suffix('.').unwrap_or(host);
    let last = host.rsplit('.').next().unwrap_or_default();
    if last.is_empty() {
        return false;
    }
    if last.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    let hex = last.strip_prefix("0x").or_else(|| last.strip_prefix("0X"));
    hex.is_some_and(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
}
