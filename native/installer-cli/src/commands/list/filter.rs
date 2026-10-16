/// Case-insensitive substring match used by the `--soc` / `--device` /
/// `--variant` filters.
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}
