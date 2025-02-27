/// Removes leading zeros from a hex string that doesn't start with "0x"
pub fn trim_leading_zeros(hex_string: &str) -> String {
    if hex_string.is_empty() {
        return String::new();
    }

    // Find the position of the first non-zero character
    match hex_string.find(|c| c != '0') {
        Some(pos) => hex_string[pos..].to_string(),
        None => "0".to_string(), // If the string contains only zeros, return "0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_leading_zeros() {
        assert_eq!(
            trim_leading_zeros("08237e9b2e5ade3da3e58e839a16f30ff3ce842711301594d966afffecfba0ec"),
            "8237e9b2e5ade3da3e58e839a16f30ff3ce842711301594d966afffecfba0ec"
        );

        assert_eq!(
            trim_leading_zeros("8237e9b2e5ade3da3e58e839a16f30ff3ce842711301594d966afffecfba0ec"),
            "8237e9b2e5ade3da3e58e839a16f30ff3ce842711301594d966afffecfba0ec"
        );

        assert_eq!(trim_leading_zeros(""), "");
        assert_eq!(trim_leading_zeros("0"), "0");
        assert_eq!(trim_leading_zeros("00"), "0");
        assert_eq!(trim_leading_zeros("0000"), "0");
        assert_eq!(trim_leading_zeros("00123"), "123");
    }
}
