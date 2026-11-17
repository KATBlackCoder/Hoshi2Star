/// A UTC date and time of day (Gregorian calendar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivilTime {
    pub year: u64,
    pub month: u64,
    pub day: u64,
    pub hour: u64,
    pub minute: u64,
    pub second: u64,
}

/// The UTC date and time `secs` seconds after the Unix epoch.
pub fn civil_from_unix(secs: u64) -> CivilTime {
    let second = secs % 60;
    let minute = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;
    let days = secs / 86400;
    // Gregorian civil date from days since Unix epoch
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    CivilTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

/// The current UTC date and time.
///
/// Uses `std::time::SystemTime` — no `chrono` dependency.
pub fn civil_now() -> CivilTime {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil_from_unix(secs)
}

/// Return the current UTC time as an ISO 8601 string: `"YYYY-MM-DDTHH:MM:SSZ"`.
pub fn now_iso8601() -> String {
    let CivilTime {
        year: yr,
        month: mon,
        day: d,
        hour,
        minute: min,
        second: sec,
    } = civil_now();
    format!("{yr:04}-{mon:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn civil(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
        let t = civil_from_unix(secs);
        (t.year, t.month, t.day, t.hour, t.minute, t.second)
    }

    #[test]
    fn civil_dates_from_unix_seconds() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        // Leap day of a year divisible by 400.
        assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0, 0));
        assert_eq!(civil(951_868_799), (2000, 2, 29, 23, 59, 59));
        assert_eq!(civil(951_868_800), (2000, 3, 1, 0, 0, 0));
        // End of a year, then the next one.
        assert_eq!(civil(1_735_689_599), (2024, 12, 31, 23, 59, 59));
        assert_eq!(civil(1_735_689_600), (2025, 1, 1, 0, 0, 0));
        // 2100 is not a leap year.
        assert_eq!(civil(4_107_542_400), (2100, 3, 1, 0, 0, 0));
        assert_eq!(civil(4_107_456_000), (2100, 2, 28, 0, 0, 0));
    }
}
