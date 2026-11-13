//! Shared display-formatting helpers used by the TUI client and the web app.
//! Pure string formatting — no UI dependencies.

use chrono::{DateTime, Utc};

/// Formats an integer with thousands separators (e.g. `1234567 → "1,234,567"`).
pub fn group_thousands(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if neg {
        out.push('-');
    }
    for (i, c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c as char);
    }
    out
}

/// Compact human-readable form (e.g. `14_089_859_944_192 → "14.09T"`). Uses 1
/// decimal for values ≥100 of their unit, 2 below; trailing zeros are trimmed.
/// Values under 1000 are shown in full.
pub fn abbreviate(n: i64) -> String {
    let neg = n < 0;
    let v = n.unsigned_abs();
    let (scaled, suffix) = if v >= 1_000_000_000_000 {
        (v as f64 / 1e12, "T")
    } else if v >= 1_000_000_000 {
        (v as f64 / 1e9, "B")
    } else if v >= 1_000_000 {
        (v as f64 / 1e6, "M")
    } else if v >= 1_000 {
        (v as f64 / 1e3, "K")
    } else {
        return if neg { format!("-{v}") } else { v.to_string() };
    };
    let decimals = if scaled >= 100.0 { 1 } else { 2 };
    let mut s = format!("{scaled:.decimals$}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if neg {
        format!("-{s}{suffix}")
    } else {
        format!("{s}{suffix}")
    }
}

/// Credits shorthand: abbreviated amount with a `CR` suffix (e.g. `"1.23B CR"`).
pub fn credits(n: i64) -> String {
    format!("{} CR", abbreviate(n))
}

/// `Some("x")` → `"x"`; `None`/empty → an em-dash placeholder.
pub fn opt(v: &Option<String>) -> String {
    v.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| "—".into())
}

/// Population as a grouped number, or an em-dash when unpopulated/unknown.
pub fn fmt_pop(p: Option<i64>) -> String {
    match p {
        Some(n) if n > 0 => group_thousands(n),
        _ => "—".into(),
    }
}

/// Standard "last updated" timestamp display: `YYYY-MM-DD HH:MM UTC`. Used wherever
/// the web app surfaces how fresh a dataset is, so every page formats it the same way.
pub fn fmt_ts(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// Like [`fmt_ts`] but renders an em-dash placeholder when the timestamp is unknown.
pub fn fmt_ts_opt(t: Option<DateTime<Utc>>) -> String {
    t.map(fmt_ts).unwrap_or_else(|| "—".into())
}

/// Body name with the leading system name stripped (`"Sol A 1 a" → "A 1 a"`);
/// falls back to the full name (e.g. the main star whose name equals the system).
pub fn short_name(name: &str, system: &str) -> String {
    name.strip_prefix(system)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(name)
        .to_string()
}
