//! Tyczenie metodą biegunową: elementy do wyniesienia punktu ze stanowiska zorientowanego na nawiązanie.
//!
//! Kąty w radianach; odczyt koła poziomego rośnie zgodnie z ruchem wskazówek zegara.

use crate::azimuth_distance;
use crate::tachymetry::EARTH_RADIUS;
use pikieta_core::Point2;
use std::f64::consts::TAU;

/// Elementy tyczenia jednego punktu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StakeoutElements {
    /// Azymut ze stanowiska na punkt.
    pub azimuth: f64,
    /// Odczyt koła poziomego do ustawienia (przy odczycie `ref_reading` na nawiązaniu).
    pub reading: f64,
    /// Kąt od nawiązania do punktu (zgodnie z ruchem wskazówek zegara).
    pub angle: f64,
    /// Długość ze współrzędnych (w układzie).
    pub grid: f64,
    /// Długość do odmierzenia w terenie (po odwróceniu redukcji); równa `grid`, gdy bez redukcji.
    pub terrain: f64,
}

/// Odwrócenie redukcji: długość w układzie na poziomą w terenie (`scale` to skala odwzorowania
/// celowej, `height` wysokość średnia celowej).
pub fn grid_to_terrain(grid: f64, scale: f64, height: Option<f64>) -> f64 {
    let ellipsoid = grid / scale;
    match height {
        Some(h) => ellipsoid * (EARTH_RADIUS + h) / EARTH_RADIUS,
        None => ellipsoid,
    }
}

/// Elementy tyczenia punktu `target` ze stanowiska `station` zorientowanego na `reference`
/// z odczytem `ref_reading`. `None`, gdy punkt lub nawiązanie pokrywa się ze stanowiskiem.
pub fn stakeout(
    station: Point2,
    reference: Point2,
    ref_reading: f64,
    target: Point2,
) -> Option<StakeoutElements> {
    let (az_ref, _) = azimuth_distance(station, reference)?;
    let (azimuth, grid) = azimuth_distance(station, target)?;
    let angle = (azimuth - az_ref).rem_euclid(TAU);
    Some(StakeoutElements {
        azimuth,
        reading: (ref_reading + angle).rem_euclid(TAU),
        angle,
        grid,
        terrain: grid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tachymetry::{shot, Shot, ShotOptions, Station};
    use pikieta_core::{grad_to_rad, rad_to_grad};

    #[test]
    fn elements_relative_to_reference() {
        let st = Point2::new(1000.0, 1000.0);
        let reference = Point2::new(1100.0, 1000.0); // na północ
        let target = Point2::new(1000.0, 1050.0); // na wschód, 50 m
        let e = stakeout(st, reference, grad_to_rad(10.0), target).unwrap();
        assert!((rad_to_grad(e.angle) - 100.0).abs() < 1e-9);
        assert!((rad_to_grad(e.reading) - 110.0).abs() < 1e-9);
        assert!((e.grid - 50.0).abs() < 1e-12);
        // Odczyt przechodzi przez 400g.
        let e = stakeout(st, reference, grad_to_rad(350.0), target).unwrap();
        assert!((rad_to_grad(e.reading) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn terrain_distance_inverts_tachymetry_reductions() {
        // Pikieta wyniesiona elementami tyczenia trafia z powrotem w punkt (redukcje w obie strony).
        let (scale, h) = (0.999923, 250.0);
        let st = Point2::new(5_000_000.0, 7_500_000.0);
        let reference = Point2::new(5_000_500.0, 7_500_000.0);
        let target = Point2::new(5_000_300.0, 7_500_400.0);
        let e = stakeout(st, reference, 0.0, target).unwrap();
        let terrain = grid_to_terrain(e.grid, scale, Some(h));
        assert!(terrain > e.grid);
        let o = ShotOptions {
            curvature: false,
            to_ellipsoid: true,
            scale,
        };
        let station = Station {
            at: st,
            height: Some(h),
            instrument: 0.0,
        };
        let m = Shot {
            hz: e.reading,
            zenith: Some(grad_to_rad(100.0)),
            distance: terrain,
            target: 0.0,
        };
        // Odczyt na nawiązaniu 0 → stała orientacji równa azymutowi nawiązania (0g).
        let r = shot(&station, 0.0, &m, &o).unwrap();
        assert!((r.point.x - target.x).abs() < 1e-6 && (r.point.y - target.y).abs() < 1e-6);
    }
}
