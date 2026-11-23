//! Tachimetria: orientacja stanowiska i pikiety z kierunku, kąta zenitalnego i odległości skośnej.
//!
//! Kąty w radianach. Kierunek poziomy (odczyt koła poziomego) rośnie zgodnie z ruchem wskazówek
//! zegara, jak azymut. Kąt zenitalny liczony od zenitu (100g to celowa pozioma).

use crate::azimuth_distance;
use pikieta_core::Point2;
use std::f64::consts::{PI, TAU};

/// Średni promień Ziemi dla obszaru Polski [m].
pub const EARTH_RADIUS: f64 = 6_382_000.0;
/// Współczynnik refrakcji.
pub const REFRACTION_K: f64 = 0.13;

/// Kąt sprowadzony do zakresu (−π, π].
fn signed(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    if r > PI {
        r - TAU
    } else {
        r
    }
}

/// Orientacja stanowiska: stała orientacji (azymut zera koła) i odchyłki na nawiązaniach.
#[derive(Debug, Clone, PartialEq)]
pub struct Orientation {
    /// Stała orientacji: azymut = odczyt + stała.
    pub constant: f64,
    /// Odchyłka na każdym nawiązaniu: azymut z współrzędnych minus azymut z odczytu i stałej.
    pub residuals: Vec<f64>,
    /// Długości celowych na nawiązania [m].
    pub distances: Vec<f64>,
}

/// Orientuje stanowisko na nawiązania `(punkt, odczyt kierunku)`. Stała to średnia ważona
/// długościami celowych (dłuższa celowa daje pewniejszy azymut).
pub fn orientation(station: Point2, refs: &[(Point2, f64)]) -> Result<Orientation, String> {
    if refs.is_empty() {
        return Err("podaj co najmniej jedno nawiązanie".into());
    }
    let mut parts = Vec::with_capacity(refs.len());
    for (i, &(p, hz)) in refs.iter().enumerate() {
        let (az, d) = azimuth_distance(station, p)
            .ok_or_else(|| format!("nawiązanie {} pokrywa się ze stanowiskiem", i + 1))?;
        parts.push((az - hz, d));
    }
    // Średnia kątów przez sumę wektorów, żeby przejście przez 0/400g nie psuło wyniku.
    let (s, c) = parts.iter().fold((0.0, 0.0), |(s, c), &(a, w)| {
        (s + w * a.sin(), c + w * a.cos())
    });
    let constant = s.atan2(c).rem_euclid(TAU);
    Ok(Orientation {
        constant,
        residuals: parts.iter().map(|&(a, _)| signed(a - constant)).collect(),
        distances: parts.iter().map(|&(_, d)| d).collect(),
    })
}

/// Pomiar pikiety.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    /// Kierunek poziomy.
    pub hz: f64,
    /// Kąt zenitalny; `None` oznacza, że `distance` jest już odległością poziomą (bez wysokości).
    pub zenith: Option<f64>,
    /// Odległość skośna (albo pozioma, gdy brak kąta zenitalnego).
    pub distance: f64,
    /// Wysokość celu (lustra).
    pub target: f64,
}

/// Stanowisko tachimetru.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    pub at: Point2,
    /// Wysokość stanowiska; bez niej pikiety nie dostają wysokości.
    pub height: Option<f64>,
    /// Wysokość instrumentu.
    pub instrument: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShotOptions {
    /// Poprawka na krzywiznę Ziemi i refrakcję w przewyższeniu.
    pub curvature: bool,
    /// Redukcja odległości poziomej na elipsoidę (z wysokości średniej celowej).
    pub to_ellipsoid: bool,
    /// Skala odwzorowania celowej (1 bez redukcji odwzorowawczej).
    pub scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotResult {
    pub point: Point2,
    /// Azymut celowej.
    pub azimuth: f64,
    /// Odległość pozioma w terenie.
    pub horizontal: f64,
    /// Odległość użyta do współrzędnych (po redukcjach).
    pub reduced: f64,
    /// Przewyższenie od punktu stanowiska do punktu pikiety.
    pub dh: Option<f64>,
    pub height: Option<f64>,
}

/// Współrzędne i wysokość pikiety ze stanowiska zorientowanego stałą `constant`.
pub fn shot(st: &Station, constant: f64, m: &Shot, o: &ShotOptions) -> Result<ShotResult, String> {
    if m.distance.is_nan() || m.distance <= 0.0 {
        return Err("odległość musi być dodatnia".into());
    }
    let (horizontal, dh) = match m.zenith {
        Some(z) => {
            let d = m.distance * z.sin();
            let mut dh = m.distance * z.cos() + st.instrument - m.target;
            if o.curvature {
                dh += (1.0 - REFRACTION_K) * d * d / (2.0 * EARTH_RADIUS);
            }
            (d, Some(dh))
        }
        None => (m.distance, None),
    };
    let height = st.height.zip(dh).map(|(h, dh)| h + dh);
    let mut reduced = horizontal;
    if o.to_ellipsoid {
        // Wysokość średnia celowej: stanowisko i cel; bez wysokości redukcji nie da się policzyć.
        if let (Some(h0), Some(h1)) = (st.height, height) {
            reduced *= EARTH_RADIUS / (EARTH_RADIUS + (h0 + h1) / 2.0);
        }
    }
    if o.scale > 0.0 {
        reduced *= o.scale;
    }
    let azimuth = (m.hz + constant).rem_euclid(TAU);
    let point = Point2::new(
        st.at.x + reduced * azimuth.cos(),
        st.at.y + reduced * azimuth.sin(),
    );
    Ok(ShotResult {
        point,
        azimuth,
        horizontal,
        reduced,
        dh,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pikieta_core::{grad_to_rad, rad_to_grad};

    fn g(v: f64) -> f64 {
        grad_to_rad(v)
    }

    #[test]
    fn orientation_on_two_points_averages_and_reports_residuals() {
        let st = Point2::new(1000.0, 1000.0);
        // Nawiązania na północ (0g) i na wschód (100g); koło obrócone o 50g, drugi odczyt z błędem 20cc.
        let refs = [
            (Point2::new(1100.0, 1000.0), g(350.0)),
            (Point2::new(1000.0, 1100.0), g(50.0) - g(0.0020)),
        ];
        let o = orientation(st, &refs).unwrap();
        assert!((rad_to_grad(o.constant) - 50.0010).abs() < 1e-9);
        assert!((rad_to_grad(o.residuals[0]) + 0.0010).abs() < 1e-9);
        assert!((rad_to_grad(o.residuals[1]) - 0.0010).abs() < 1e-9);
    }

    #[test]
    fn orientation_across_zero() {
        let st = Point2::new(0.0, 0.0);
        let refs = [
            (Point2::new(100.0, 0.0), g(399.9990)),
            (Point2::new(0.0, 100.0), g(100.0010)),
        ];
        // Stałe 0,0010g i 399,9990g: średnia 0 (a nie 200g).
        let o = orientation(st, &refs).unwrap();
        assert!(signed(o.constant).abs() < 1e-12);
    }

    #[test]
    fn shot_with_zenith_angle_and_heights() {
        let st = Station {
            at: Point2::new(5_000_000.0, 7_500_000.0),
            height: Some(200.0),
            instrument: 1.55,
        };
        // Celowa na wschód, 100 m skośnie, w górę o 2g; lustro 1,80 m.
        let m = Shot {
            hz: g(100.0),
            zenith: Some(g(98.0)),
            distance: 100.0,
            target: 1.80,
        };
        let r = shot(&st, 0.0, &m, &ShotOptions::default()).unwrap();
        let d = 100.0 * g(98.0).sin();
        let dh = 100.0 * g(98.0).cos() + 1.55 - 1.80;
        assert!((r.horizontal - d).abs() < 1e-9);
        assert!((r.dh.unwrap() - dh).abs() < 1e-9);
        assert!((r.height.unwrap() - (200.0 + dh)).abs() < 1e-9);
        assert!(
            (r.point.y - (7_500_000.0 + d)).abs() < 1e-6 && (r.point.x - 5_000_000.0).abs() < 1e-6
        );
        // Krzywizna i refrakcja: (1 − k) d² / 2R ≈ 0,7 mm na 100 m.
        let c = shot(
            &st,
            0.0,
            &m,
            &ShotOptions {
                curvature: true,
                ..Default::default()
            },
        )
        .unwrap();
        let corr = c.dh.unwrap() - r.dh.unwrap();
        assert!((corr - 0.87 * d * d / (2.0 * EARTH_RADIUS)).abs() < 1e-12 && corr > 0.0006);
    }

    #[test]
    fn reductions_shorten_distance() {
        let st = Station {
            at: Point2::new(0.0, 0.0),
            height: Some(300.0),
            instrument: 0.0,
        };
        let m = Shot {
            hz: 0.0,
            zenith: Some(g(100.0)),
            distance: 1000.0,
            target: 0.0,
        };
        let o = ShotOptions {
            to_ellipsoid: true,
            scale: 0.999923,
            ..Default::default()
        };
        let r = shot(&st, 0.0, &m, &o).unwrap();
        let expected = 1000.0 * EARTH_RADIUS / (EARTH_RADIUS + 300.0) * 0.999923;
        assert!((r.reduced - expected).abs() < 1e-9);
        assert!((r.point.x - expected).abs() < 1e-9);
    }

    #[test]
    fn horizontal_distance_without_zenith_has_no_height() {
        let st = Station {
            at: Point2::new(0.0, 0.0),
            height: Some(100.0),
            instrument: 1.5,
        };
        let m = Shot {
            hz: g(200.0),
            zenith: None,
            distance: 25.0,
            target: 0.0,
        };
        let r = shot(&st, 0.0, &m, &ShotOptions::default()).unwrap();
        assert!(r.height.is_none());
        assert!((r.point.x + 25.0).abs() < 1e-9);
    }
}
