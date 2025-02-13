use std::str::FromStr;

use anyhow::Result;
use bigdecimal::{num_bigint::BigInt, FromPrimitive, Num};
use num_integer::Integer;
use rust_decimal::Decimal;
use starknet::core::types::Felt;

lazy_static::lazy_static! {
    static ref TWO_POW_128: BigInt = BigInt::from(2).pow(128);
}

#[allow(dead_code)]
pub fn to_uint256(n: BigInt) -> (Felt, Felt) {
    let (n_high, n_low) = n.div_rem(&TWO_POW_128);
    let (_, low_bytes) = n_low.to_bytes_be();
    let (_, high_bytes) = n_high.to_bytes_be();

    (
        Felt::from_bytes_be_slice(&low_bytes),
        Felt::from_bytes_be_slice(&high_bytes),
    )
}

#[allow(dead_code)]
pub fn convert_to_bigint(amount: &str, divisibility: u64) -> Result<BigInt> {
    // Parse the amount string to BigDecimal
    let decimal_amount = Decimal::from_str(amount)?;

    // Calculate the multiplicative factor from divisibility
    let factor = Decimal::from_i64(10_i64.pow(divisibility as u32))
        .ok_or_else(|| anyhow::anyhow!("Invalid divisibility factor"))?;

    // Multiply the decimal amount by the factor
    let scaled_amount = decimal_amount * factor;

    // Convert the scaled amount to BigInt (removing any fractional part)
    let bigint_result = BigInt::from_str(&scaled_amount.trunc().to_string())?;

    Ok(bigint_result)
}

#[allow(dead_code)]
pub fn hex_to_uint256(hex_str: &str) -> Result<(Felt, Felt)> {
    // Parse the hexadecimal string into a BigInt
    let n = BigInt::from_str_radix(hex_str, 16)?;

    // Split the BigInt into two 128-bit chunks (high and low)
    let (n_high, n_low) = n.div_rem(&TWO_POW_128);

    // Convert the chunks to byte arrays and then to Felt
    let (_, low_bytes) = n_low.to_bytes_be();
    let (_, high_bytes) = n_high.to_bytes_be();

    Ok((
        Felt::from_bytes_be_slice(&low_bytes),
        Felt::from_bytes_be_slice(&high_bytes),
    ))
}

#[cfg(test)]
mod tests {
    use starknet::macros::felt;

    use super::*;

    #[test]
    fn test_hex_to_uint256() {
        let hex_tx_id = "a8d6ed49c8177545d81e1aee2fabb8d75bc07ae0cf0f469d165b2ca505d5e117";

        // Digest in cairo should be equal to the value below, using hex_to_hash_rev from auto_claim
        // [0x17e1d505, 0xa52c5b16, 0x9d460fcf, 0xe07ac05b, 0xd7b8ab2f, 0xee1a1ed8, 0x457517c8, 0x49edd6a8]
        // which is equal to { low: 121959160878427944421643839789432430871, high: 224426267596249609810929133391035742423) }

        let expected_low = felt!("121959160878427944421643839789432430871");
        let expected_high = felt!("224426267596249609810929133391035742423");

        let (low, high) = hex_to_uint256(hex_tx_id).unwrap();

        assert!(low == expected_low);
        assert!(high == expected_high);
    }
}
