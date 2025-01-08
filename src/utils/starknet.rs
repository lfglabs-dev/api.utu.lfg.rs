use std::str::FromStr;

use anyhow::Result;
use bigdecimal::{num_bigint::BigInt, FromPrimitive, Num};
use num_integer::Integer;
use rust_decimal::Decimal;
use starknet::core::types::FieldElement;

lazy_static::lazy_static! {
    static ref TWO_POW_128: BigInt = BigInt::from(2).pow(128);
}

pub fn to_uint256(n: BigInt) -> (FieldElement, FieldElement) {
    let (n_high, n_low) = n.div_rem(&TWO_POW_128);
    let (_, low_bytes) = n_low.to_bytes_be();
    let (_, high_bytes) = n_high.to_bytes_be();

    (
        FieldElement::from_byte_slice_be(&low_bytes).unwrap(),
        FieldElement::from_byte_slice_be(&high_bytes).unwrap(),
    )
}

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

pub fn hex_to_uint256(hex_str: &str) -> Result<(FieldElement, FieldElement)> {
    // Parse the hexadecimal string into a BigInt
    let n = BigInt::from_str_radix(hex_str, 16)?;

    // Split the BigInt into two 128-bit chunks (high and low)
    let (n_high, n_low) = n.div_rem(&TWO_POW_128);

    // Convert the chunks to byte arrays and then to FieldElement
    let (_, low_bytes) = n_low.to_bytes_be();
    let (_, high_bytes) = n_high.to_bytes_be();

    Ok((
        FieldElement::from_byte_slice_be(&low_bytes)?,
        FieldElement::from_byte_slice_be(&high_bytes)?,
    ))
}
