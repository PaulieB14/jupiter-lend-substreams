//! Small, dependency-free helpers: little-endian borsh reader, exact decimal formatting,
//! UTC day formatting.

/// Minimal borsh reader over an Anchor event payload (after the 8-byte discriminator).
pub struct Reader<'a> {
    b: &'a [u8],
    o: usize,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Reader { b, o: 0 }
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.o + n > self.b.len() {
            return None;
        }
        let s = &self.b[self.o..self.o + n];
        self.o += n;
        Some(s)
    }
    pub fn remaining(&self) -> usize {
        self.b.len() - self.o
    }
    pub fn pubkey(&mut self) -> Option<String> {
        self.take(32).map(|s| bs58::encode(s).into_string())
    }
    pub fn bool(&mut self) -> Option<bool> {
        self.take(1).map(|s| s[0] != 0)
    }
    pub fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }
    pub fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|s| u16::from_le_bytes([s[0], s[1]]))
    }
    pub fn i16(&mut self) -> Option<i16> {
        self.take(2).map(|s| i16::from_le_bytes([s[0], s[1]]))
    }
    pub fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|s| u32::from_le_bytes(s.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> Option<i32> {
        self.take(4).map(|s| i32::from_le_bytes(s.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> Option<u64> {
        self.take(8).map(|s| u64::from_le_bytes(s.try_into().unwrap()))
    }
    pub fn u128(&mut self) -> Option<u128> {
        self.take(16).map(|s| u128::from_le_bytes(s.try_into().unwrap()))
    }
    pub fn i128(&mut self) -> Option<i128> {
        self.take(16).map(|s| i128::from_le_bytes(s.try_into().unwrap()))
    }
}

/// Exact decimal string of `raw / 10^decimals` (no floats). Empty when decimals < 0.
pub fn ui(raw: i128, decimals: i32) -> String {
    if decimals < 0 {
        return String::new();
    }
    let neg = raw < 0;
    let digits = raw.unsigned_abs().to_string();
    let d = decimals as usize;
    let s = if d == 0 {
        digits
    } else if digits.len() > d {
        let (i, f) = digits.split_at(digits.len() - d);
        format!("{}.{}", i, f)
    } else {
        format!("0.{}{}", "0".repeat(d - digits.len()), digits)
    };
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    if neg && s != "0" {
        format!("-{}", s)
    } else {
        s
    }
}

/// UTC calendar day `YYYY-MM-DD` for a unix timestamp (Howard Hinnant's civil_from_days).
pub fn utc_day(ts: i64) -> String {
    let z = ts.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ui_formatting() {
        assert_eq!(ui(1_000_000, 6), "1");
        assert_eq!(ui(-16_650_247, 6), "-16.650247");
        assert_eq!(ui(5, 9), "0.000000005");
        assert_eq!(ui(0, 6), "0");
        assert_eq!(ui(123, 0), "123");
        assert_eq!(ui(123, -1), "");
        assert_eq!(ui(i128::MIN + 1, 6).starts_with('-'), true);
    }
    #[test]
    fn days() {
        assert_eq!(utc_day(0), "1970-01-01");
        assert_eq!(utc_day(1753448175), "2025-07-25");
        assert_eq!(utc_day(1709251199), "2024-02-29");
        assert_eq!(utc_day(1709251200), "2024-03-01");
    }
}
