//! Obliczenia geodezyjne na płaszczyźnie.

use pikieta_core::Point2;
use std::f64::consts::TAU;

/// Azymut (w radianach, zakres `[0, 2π)`, liczony od północy zgodnie z ruchem
/// wskazówek zegara) i długość odcinka `a → b`.
///
/// Dla punktów pokrywających się zwraca `None`, bo azymut jest nieokreślony.
pub fn azimuth_distance(a: Point2, b: Point2) -> Option<(f64, f64)> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    let azimuth = dy.atan2(dx).rem_euclid(TAU);
    Some((azimuth, dx.hypot(dy)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pikieta_core::rad_to_grad;

    fn az_grad(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
        let (az, d) = azimuth_distance(Point2::new(a.0, a.1), Point2::new(b.0, b.1)).unwrap();
        (rad_to_grad(az), d)
    }

    #[test]
    fn cardinal_directions() {
        let cases = [
            ((100.0, 0.0), 0.0),
            ((0.0, 100.0), 100.0),
            ((-100.0, 0.0), 200.0),
            ((0.0, -100.0), 300.0),
        ];
        for (b, expected) in cases {
            let (az, d) = az_grad((0.0, 0.0), b);
            assert!((az - expected).abs() < 1e-9, "{b:?}: {az}");
            assert!((d - 100.0).abs() < 1e-9);
        }
    }

    #[test]
    fn diagonal() {
        let (az, d) = az_grad((1000.0, 2000.0), (1300.0, 2400.0));
        assert!((d - 500.0).abs() < 1e-9);
        // atan(400/300) = 59.0334g
        assert!((az - 59.033447).abs() < 1e-6, "{az}");
    }

    #[test]
    fn coincident_points() {
        let p = Point2::new(5.0, 5.0);
        assert_eq!(azimuth_distance(p, p), None);
    }
}
