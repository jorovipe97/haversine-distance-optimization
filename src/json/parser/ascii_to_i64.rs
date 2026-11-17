#[repr(u8)]
enum ParsingState {
    ParsingCoefficient,
    ParsingExponent,
}

const BASE: i64 = 10;

pub fn ascii_to_i64(text: String) -> i64 {
    // TOOD: Use SWAR (Simd withing a register) which is O(1)
    // coefficient x base.pow(exponent)
    let mut coefficient: i64 = 0;
    let mut exponent: u32 = 0;
    let mut is_negative = false;

    let mut state = ParsingState::ParsingCoefficient;

    for b in text.bytes() {
        if b == b'-' {
            is_negative = true;
            continue;
        }

        if b == b'e' || b == b'E' {
            state = ParsingState::ParsingExponent;
            continue;
        }

        match state {
            ParsingState::ParsingCoefficient => {
                coefficient = (coefficient * 10) + (b - b'0') as i64;
            }
            ParsingState::ParsingExponent => exponent = (exponent * 10) + (b - b'0') as u32,
        }
    }

    if is_negative {
        coefficient = -1 * coefficient;
    }

    if exponent != 0 {
        return coefficient * BASE.pow(exponent);
    }

    coefficient
}
