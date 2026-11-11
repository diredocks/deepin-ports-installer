//! Case-insensitive substring matching shared by filters and resolution.

/// Whether `haystack` contains `needle`, ignoring case.
///
/// An empty `needle` always matches.
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    needle.is_empty() || haystack.to_lowercase().contains(&needle.to_lowercase())
}
