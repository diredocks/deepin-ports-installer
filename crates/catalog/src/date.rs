//! Parsing, comparing and formatting of image build dates.
//!
//! Build dates are embedded in file names as `YYYYMMDD-HHMMSS`
//! (e.g. `20250625-030117`). Filters may pass only the `YYYYMMDD` day, possibly
//! with separators such as `2025-06-25`, so comparisons are made on the digits
//! alone.

/// Whether `token` is a plausible `YYYYMMDD` day, e.g. `20250625`.
pub fn is_day(token: &str) -> bool {
    token.len() == 8 && token.starts_with("20") && token.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether `token` is a `HHMMSS` time, e.g. `030117`.
pub fn is_time(token: &str) -> bool {
    token.len() == 6 && token.bytes().all(|byte| byte.is_ascii_digit())
}

/// The `YYYYMMDD-HHMMSS` timestamp embedded in a file `stem`, if any.
///
/// The stem is split on `-` and scanned for a day token immediately followed by
/// a six-digit time token; trailing suffixes such as `_kernel-6.7` are ignored.
pub fn from_stem(stem: &str) -> Option<String> {
    let tokens: Vec<&str> = stem.split('-').collect();

    tokens.windows(2).find_map(|pair| {
        let time = pair[1].split('_').next().unwrap_or(pair[1]);
        (is_day(pair[0]) && is_time(time)).then(|| format!("{}-{time}", pair[0]))
    })
}

/// Digits-only form used to compare dates regardless of separators
/// (`2025-06-25` and `20250625` both become `20250625`).
fn digits(date: &str) -> String {
    date.bytes()
        .filter(|byte| byte.is_ascii_digit())
        .map(char::from)
        .collect()
}

/// Whether `date` falls on the day named by `day`, ignoring separators and any
/// time component.
pub fn is_on_day(date: &str, day: &str) -> bool {
    digits(date).starts_with(&digits(day))
}

/// Render `date` as `2025-06-25 03:01:17` when it holds a full timestamp,
/// otherwise return it unchanged.
pub fn format(date: &str) -> String {
    let digits = digits(date);
    if digits.len() == 14 {
        format!(
            "{}-{}-{} {}:{}:{}",
            &digits[0..4],
            &digits[4..6],
            &digits[6..8],
            &digits[8..10],
            &digits[10..12],
            &digits[12..14]
        )
    } else {
        date.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_times_are_recognized() {
        assert!(is_day("20250625"));
        assert!(!is_day("2025062"));
        assert!(!is_day("19990625"));
        assert!(!is_day("2025062a"));

        assert!(is_time("030117"));
        assert!(!is_time("03011"));
        assert!(!is_time("03011a"));
    }

    #[test]
    fn timestamps_are_extracted_from_stems() {
        assert_eq!(
            from_stem("deepin-25-crimson-preview-riscv64-jh7110-20250625-030117"),
            Some("20250625-030117".to_owned())
        );
        assert_eq!(
            from_stem("deepin-25-crimson-preview-riscv64-jh7110-20250806-180508_kernel-6.7"),
            Some("20250806-180508".to_owned())
        );
        assert_eq!(from_stem("deepin-no-date"), None);
    }

    #[test]
    fn days_match_ignoring_separators_and_time() {
        assert!(is_on_day("20250625-030117", "20250625"));
        assert!(is_on_day("20250625-030117", "2025-06-25"));
        assert!(!is_on_day("20250625-030117", "20250626"));
    }

    #[test]
    fn full_timestamps_are_made_readable() {
        assert_eq!(format("20250625-030117"), "2025-06-25 03:01:17");
        assert_eq!(format("2025-06-25"), "2025-06-25");
        assert_eq!(format("unknown"), "unknown");
    }
}
