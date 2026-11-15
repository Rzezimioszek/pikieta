//! Wiązania WebAssembly: cienka warstwa nad crate'ami rdzenia.

use pikieta_core::{rad_to_grad, Point2};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct AzimuthDistance {
    /// Azymut w gradach, zakres [0, 400).
    pub azimuth_grad: f64,
    /// Długość w metrach.
    pub distance: f64,
}

/// Azymut i długość odcinka A → B. `x` to północ, `y` to wschód.
/// Zwraca `undefined`, gdy punkty się pokrywają.
#[wasm_bindgen]
pub fn azimuth_distance(xa: f64, ya: f64, xb: f64, yb: f64) -> Option<AzimuthDistance> {
    pikieta_calc::azimuth_distance(Point2::new(xa, ya), Point2::new(xb, yb)).map(|(az, d)| {
        AzimuthDistance {
            azimuth_grad: rad_to_grad(az),
            distance: d,
        }
    })
}

/// Wersja rdzenia, do pokazania w interfejsie.
#[wasm_bindgen]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
