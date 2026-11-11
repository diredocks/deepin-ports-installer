//! Case-insensitive substring matching shared by filters and resolution.

/// Whether `haystack` contains `needle`, ignoring case.
///
/// An empty `needle` always matches.
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    needle.is_empty() || haystack.to_lowercase().contains(&needle.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_ignoring_case() {
        assert!(contains_ignore_case("25-Crimson-Preview", "crimson"));
        assert!(contains_ignore_case("jh7110", "JH7110"));
        assert!(contains_ignore_case("anything", ""));
        assert!(!contains_ignore_case("jh7110", "eic7700"));
    }
}
