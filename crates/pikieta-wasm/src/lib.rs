//! Wiązania WebAssembly: cienka warstwa nad crate'ami rdzenia.

use pikieta_core::{rad_to_grad, Point2};
use pikieta_crs::Crs;
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

/// Przelicza punkt między układami płaskimi podanymi kodami EPSG.
/// Zwraca `[x, y]` albo `undefined` dla nieobsługiwanego układu.
#[wasm_bindgen]
pub fn transform(from_epsg: u32, to_epsg: u32, x: f64, y: f64) -> Option<Vec<f64>> {
    let from = Crs::from_epsg(from_epsg)?;
    let to = Crs::from_epsg(to_epsg)?;
    let p = pikieta_crs::transform(Point2::new(x, y), from, to);
    Some(vec![p.x, p.y])
}

/// Współrzędne płaskie na geodezyjne. Zwraca `[szerokość, długość]` w stopniach
/// albo `undefined` dla nieobsługiwanego układu.
#[wasm_bindgen]
pub fn to_geodetic(epsg: u32, x: f64, y: f64) -> Option<Vec<f64>> {
    let g = Crs::from_epsg(epsg)?.inverse(Point2::new(x, y));
    Some(vec![g.lat, g.lon])
}

/// Redukcje odwzorowawcze odcinka A → B w układzie o podanym kodzie EPSG.
/// Zwraca `[skala odcinka, długość na elipsoidzie, zbieżność południków w A (grady)]`
/// albo `undefined` dla nieobsługiwanego układu.
#[wasm_bindgen]
pub fn line_reduction(epsg: u32, xa: f64, ya: f64, xb: f64, yb: f64) -> Option<Vec<f64>> {
    let crs = Crs::from_epsg(epsg)?;
    let (a, b) = (Point2::new(xa, ya), Point2::new(xb, yb));
    Some(vec![
        crs.line_scale(a, b),
        crs.ellipsoidal_distance(a, b),
        rad_to_grad(crs.convergence(a)),
    ])
}
