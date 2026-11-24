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

/// Whole minutes from `t` until now (0 for a time in the future).
pub fn minutes_since(t: DateTime<Utc>) -> i64 {
    (Utc::now() - t).num_minutes().max(0)
}

/// An age in minutes as "35m" / "5h" / "3d" (`short`) or "35 min ago" / "5 h ago" /
/// "3 days ago". Hours run up to two days, days after that.
pub fn fmt_age(mins: i64, short: bool) -> String {
    let mins = mins.max(0);
    let (n, unit_short, unit_long) = match mins {
        0..=59 => (mins, "m", " min"),
        60..=2879 => (mins / 60, "h", " h"),
        _ => (mins / 1440, "d", " days"),
    };
    if short { format!("{n}{unit_short}") } else { format!("{n}{unit_long} ago") }
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

/// The Sun's radius in metres (IAU nominal), for star radii in solar radii.
pub const SOLAR_RADIUS_M: f64 = 695_700_000.0;

/// A star's radius (metres, as the journal gives it) for a table: in solar radii, or in
/// kilometres below 0.05 R☉, where two decimals say little (white dwarfs, neutron stars, black holes):
/// `"0.56 R☉"`, `"42.9 R☉"`, `"442 R☉"`, `"8,371 km"`.
pub fn star_radius(metres: f32) -> String {
    let m = f64::from(metres);
    let sol = m / SOLAR_RADIUS_M;
    if sol < 0.05 {
        format!("{} km", group_thousands((m / 1000.0).round() as i64))
    } else if sol < 10.0 {
        format!("{sol:.2} R☉")
    } else if sol < 100.0 {
        format!("{sol:.1} R☉")
    } else {
        format!("{} R☉", group_thousands(sol.round() as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_age_buckets() {
        assert_eq!(fmt_age(35, true), "35m");
        assert_eq!(fmt_age(35, false), "35 min ago");
        assert_eq!(fmt_age(5 * 60, true), "5h");
        assert_eq!(fmt_age(47 * 60, false), "47 h ago");
        assert_eq!(fmt_age(3 * 1440, true), "3d");
        assert_eq!(fmt_age(3 * 1440, false), "3 days ago");
        assert_eq!(fmt_age(-5, true), "0m");
    }

    #[test]
    fn star_radii_read_in_solar_radii_or_km() {
        assert_eq!(super::star_radius(389_600_000.0), "0.56 R☉");
        assert_eq!(super::star_radius(29_850_000_000.0), "42.9 R☉");
        assert_eq!(super::star_radius(307_700_000_000.0), "442 R☉");
        assert_eq!(super::star_radius(8_371_000.0), "8,371 km");
        assert_eq!(super::star_radius(11_000.0), "11 km");
    }
}
