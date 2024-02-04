pub(crate) fn round(value: f64, decimals: u32) -> f64 {
    let y = 10i32.pow(decimals) as f64;
    (value * y).round() / y
}
