pub(crate) fn round(value: f64, decimals: u32) -> f64 {
    let y = 10i32.pow(decimals) as f64;
    (value * y).round() / y
}

#[cfg(test)]
mod tests {
    use super::round;

    #[test]
    fn round_three_decimals_rounds_half_up() {
        assert_eq!(round(84.2549, 3), 84.255);
    }

    #[test]
    fn round_repeating_fraction_truncates_to_three_decimals() {
        assert_eq!(round(1.0 / 3.0, 3), 0.333);
    }

    #[test]
    fn round_nan_returns_nan() {
        assert!(round(f64::NAN, 3).is_nan());
    }
}
