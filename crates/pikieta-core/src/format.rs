//! Zaokrąglanie i formatowanie wyników.
//!
//! Geodezyjna reguła zaokrąglania: gdy odrzucana część to dokładnie pół jednostki
//! ostatniego miejsca, ostatnia zachowana cyfra ma być parzysta (12,345 → 12,34;
//! 12,355 → 12,36). Reguła działa na zapisie dziesiętnym liczby, nie na jej
//! reprezentacji binarnej, więc 12,345 wpisane przez użytkownika daje 12,34,
//! choć w pamięci jest to 12,34499999…

/// Liczba miejsc po przecinku dla wielkości pokazywanych w wynikach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Precision {
    /// Współrzędne i wysokości [m].
    pub coord: usize,
    /// Długości i miary [m].
    pub length: usize,
    /// Kąty i azymuty [g].
    pub angle: usize,
    /// Pola [m²].
    pub area: usize,
}

impl Default for Precision {
    fn default() -> Self {
        Self {
            coord: 2,
            length: 3,
            angle: 4,
            area: 2,
        }
    }
}

/// Zaokrągla do `decimals` miejsc po przecinku regułą „do parzystej” i zwraca
/// zapis z kropką dziesiętną, zawsze z pełną liczbą miejsc.
pub fn round_half_even(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    // Display dla f64 daje najkrótszy zapis dziesiętny bez wykładnika, który
    // po wczytaniu odtwarza tę samą liczbę.
    let text = value.abs().to_string();
    let (int_part, frac_part) = text.split_once('.').unwrap_or((&text, ""));

    let mut digits: Vec<u8> = int_part.bytes().map(|b| b - b'0').collect();
    let int_len = digits.len();
    let frac: Vec<u8> = frac_part.bytes().map(|b| b - b'0').collect();
    digits.extend(frac.iter().take(decimals));
    digits.resize(int_len + decimals, 0);

    if let Some(&next) = frac.get(decimals) {
        let rest_nonzero = frac[decimals + 1..].iter().any(|&d| d != 0);
        let last_odd = digits.last().is_some_and(|d| d % 2 == 1);
        if next > 5 || (next == 5 && (rest_nonzero || last_odd)) {
            increment(&mut digits);
        }
    }

    let int_len = digits.len() - decimals;
    let mut out = String::with_capacity(digits.len() + 2);
    if value.is_sign_negative() && digits.iter().any(|&d| d != 0) {
        out.push('-');
    }
    out.extend(digits[..int_len].iter().map(|&d| char::from(b'0' + d)));
    if decimals > 0 {
        out.push('.');
        out.extend(digits[int_len..].iter().map(|&d| char::from(b'0' + d)));
    }
    out
}

/// Dodaje jedność na ostatniej pozycji, z przeniesieniem (9,99 → 10,00).
fn increment(digits: &mut Vec<u8>) {
    for d in digits.iter_mut().rev() {
        if *d == 9 {
            *d = 0;
        } else {
            *d += 1;
            return;
        }
    }
    digits.insert(0, 1);
}

#[cfg(test)]
mod tests {
    use super::round_half_even as r;

    #[test]
    fn half_goes_to_even() {
        assert_eq!(r(12.345, 2), "12.34");
        assert_eq!(r(12.355, 2), "12.36");
        assert_eq!(r(0.125, 2), "0.12");
        assert_eq!(r(0.135, 2), "0.14");
        assert_eq!(r(2.5, 0), "2");
        assert_eq!(r(3.5, 0), "4");
        assert_eq!(r(5788456.485, 2), "5788456.48");
        assert_eq!(r(5788456.495, 2), "5788456.50");
    }

    #[test]
    fn more_than_half_rounds_up() {
        assert_eq!(r(12.3451, 2), "12.35");
        assert_eq!(r(12.346, 2), "12.35");
        assert_eq!(r(12.344999, 2), "12.34");
    }

    #[test]
    fn carry_padding_and_sign() {
        assert_eq!(r(9.995, 2), "10.00");
        assert_eq!(r(99.96, 1), "100.0");
        assert_eq!(r(7.0, 3), "7.000");
        assert_eq!(r(1.5, 4), "1.5000");
        assert_eq!(r(-1.005, 2), "-1.00");
        assert_eq!(r(-1.015, 2), "-1.02");
        assert_eq!(r(-0.004, 2), "0.00");
        assert_eq!(r(0.0, 2), "0.00");
        assert_eq!(r(1e-7, 3), "0.000");
        assert_eq!(r(123.0, 0), "123");
    }

    #[test]
    fn computed_values_are_not_ties() {
        // 0,1 + 0,2 = 0,30000000000000004: to nie jest dokładnie połowa.
        assert_eq!(r(0.1 + 0.2, 1), "0.3");
        assert_eq!(r(1.0 / 3.0, 4), "0.3333");
        assert_eq!(r(f64::NAN, 2), "NaN");
    }
}
