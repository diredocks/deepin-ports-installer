//! Case-insensitive substring matching shared by filters and resolution.

pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    needle.is_empty() || haystack.to_lowercase().contains(&needle.to_lowercase())
}
