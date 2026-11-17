#[repr(u8)]
enum ParsingState {
    ParsingCoefficientIntegerPart,
    ParsingCoefficientFractionalPart,
    ParsingExponent,
}

const BASE: f64 = 10.0;

// This is a hard problem https://stackoverflow.com/questions/85223/how-to-manually-parse-a-floating-point-number-from-a-string
// https://web.archive.org/web/20080406035949/http://docs.sun.com/source/806-3568/ncg_goldberg.html#680
// https://dl.acm.org/doi/epdf/10.1145/93548.93557
// Here I implement a naive solution to see the problems in action and then I will dig deeper.
pub fn ascii_to_f64(text: String) -> f64 {
    // coefficient x base.pow(exponent)
    let mut coefficient = 0.0;
    let mut decimal = 1.0;
    let mut is_coefficient_negative = false;
    let mut exponent: i32 = 0;
    let mut is_exponent_negative = false;
    let mut state = ParsingState::ParsingCoefficientIntegerPart;

    for b in text.bytes() {
        if b == b'.' {
            state = ParsingState::ParsingCoefficientFractionalPart;
            continue;
        }

        if b == b'e' || b == b'E' {
            state = ParsingState::ParsingExponent;
            continue;
        }

        match state {
            ParsingState::ParsingCoefficientIntegerPart => {
                if b == b'-' {
                    is_coefficient_negative = true;
                    continue;
                }

                let digit = (b - b'0') as f64;
                coefficient = (coefficient * 10.0) + digit
            }
            ParsingState::ParsingCoefficientFractionalPart => {
                decimal = decimal * 10.0;
                let digit = (b - b'0') as f64;
                coefficient = coefficient + (digit / decimal)
            }
            ParsingState::ParsingExponent => {
                if b == b'-' {
                    is_exponent_negative = true;
                    continue;
                }

                let digit = (b - b'0') as i32;
                exponent = (exponent * 10) + digit;
            }
        }
    }

    if is_coefficient_negative {
        coefficient = -coefficient;
    }

    if exponent != 0 {
        if is_exponent_negative {
            exponent = -exponent;
        }

        return coefficient * BASE.powi(exponent);
    }

    coefficient
}
