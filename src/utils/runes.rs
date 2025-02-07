use starknet::core::types::FieldElement;

#[allow(dead_code)]
pub fn symbol_as_felt(symbol: String) -> FieldElement {
    let bytes = symbol.as_bytes();
    let mut rune_id_felt: u128 = 0;
    let mut shift_amount: u128 = 1;

    for &byte in bytes.iter() {
        rune_id_felt += (byte as u128) * shift_amount;
        shift_amount *= 256;
    }

    FieldElement::from(rune_id_felt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_as_felt() {
        let symbol = "🐕";
        let symbol_felt = symbol_as_felt(symbol.to_string());
        let expected_res = FieldElement::from_dec_str("2509283312").unwrap();
        assert_eq!(symbol_felt, expected_res);
    }
}
