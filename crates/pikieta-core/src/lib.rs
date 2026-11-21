//! Podstawowe typy geodezyjne.
//!
//! Konwencja osi jest geodezyjna: `x` rośnie na północ, `y` na wschód.

use std::f64::consts::PI;

mod format;
pub use format::{round_half_even, Precision};

/// Punkt w układzie płaskim (np. PL-2000), współrzędne w metrach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    /// Północ (northing).
    pub x: f64,
    /// Wschód (easting).
    pub y: f64,
}

impl Point2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Radiany na grady (400g = 2π).
pub fn rad_to_grad(rad: f64) -> f64 {
    rad * 200.0 / PI
}

/// Grady na radiany.
pub fn grad_to_rad(grad: f64) -> f64 {
    grad * PI / 200.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grad_round_trip() {
        assert!((rad_to_grad(PI) - 200.0).abs() < 1e-12);
        assert!((grad_to_rad(rad_to_grad(1.2345)) - 1.2345).abs() < 1e-12);
    }
}
