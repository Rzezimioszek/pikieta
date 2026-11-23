//! Wcięcie wstecz: położenie stanowiska z kierunków na punkty znane.
//!
//! Trzy kierunki: rozwiązanie ścisłe jako drugie przecięcie dwóch okręgów (każdy przez parę punktów
//! znanych, z kątem wpisanym równym kątowi zmierzonemu). Więcej kierunków: wyrównanie metodą
//! najmniejszych kwadratów (niewiadome X, Y i stała orientacji), z rozwiązania z trzech punktów.
//!
//! Punkty traktujemy jak liczby zespolone x + i·y (x północ, y wschód). Wtedy azymut geodezyjny
//! to argument liczby, a kąt zgodny z ruchem wskazówek zegara na mapie jest kątem dodatnim.

use crate::azimuth_distance;
use pikieta_core::Point2;
use std::f64::consts::{PI, TAU};

fn signed(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    if r > PI {
        r - TAU
    } else {
        r
    }
}

/// Środek okręgu, na którym punkt widzi cięciwę A→B pod kątem `alpha` (od kierunku na A do kierunku na B).
fn arc_center(a: Point2, b: Point2, alpha: f64) -> Option<Point2> {
    let s = alpha.sin();
    if s.abs() < 1e-9 {
        return None;
    }
    let k = alpha.cos() / s / 2.0;
    // M = (A + B)/2 + i·(B − A)/2·ctg α
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    Some(Point2::new(
        (a.x + b.x) / 2.0 - dy * k,
        (a.y + b.y) / 2.0 + dx * k,
    ))
}

/// Odbicie punktu `p` względem prostej przez `m1` i `m2`.
fn reflect(p: Point2, m1: Point2, m2: Point2) -> Option<Point2> {
    let (ux, uy) = (m2.x - m1.x, m2.y - m1.y);
    let l2 = ux * ux + uy * uy;
    if l2 < 1e-18 {
        return None;
    }
    let t = ((p.x - m1.x) * ux + (p.y - m1.y) * uy) / l2;
    let (fx, fy) = (m1.x + t * ux, m1.y + t * uy);
    Some(Point2::new(2.0 * fx - p.x, 2.0 * fy - p.y))
}

/// Wynik wcięcia wstecz.
#[derive(Debug, Clone, PartialEq)]
pub struct ResectionResult {
    pub point: Point2,
    /// Stała orientacji: azymut = odczyt + stała.
    pub orientation: f64,
    /// Odchyłki kierunków (azymut ze współrzędnych minus odczyt i stała).
    pub residuals: Vec<f64>,
    /// Błąd średni kierunku m₀ [rad] (tylko przy nadliczbowych obserwacjach).
    pub m0: Option<f64>,
    /// Błędy średnie współrzędnych mx, my [m] (tylko przy nadliczbowych obserwacjach).
    pub mxy: Option<(f64, f64)>,
    /// Odległość stanowiska od okręgu przez trzy pierwsze punkty, względem jego promienia.
    /// Bliska zeru oznacza położenie na „okręgu niebezpiecznym” (rozwiązanie nieoznaczone).
    pub danger: f64,
}

/// Wcięcie wstecz z kierunków (odczytów koła poziomego) `(punkt znany, odczyt)`, co najmniej trzech.
pub fn resection(obs: &[(Point2, f64)]) -> Result<ResectionResult, String> {
    if obs.len() < 3 {
        return Err("wcięcie wstecz wymaga co najmniej trzech punktów".into());
    }
    let (a, b, c) = (obs[0], obs[1], obs[2]);
    let alpha = (b.1 - a.1).rem_euclid(TAU);
    let beta = (c.1 - b.1).rem_euclid(TAU);
    let m1 = arc_center(a.0, b.0, alpha)
        .ok_or("stanowisko leży na prostej przez dwa pierwsze punkty")?;
    let m2 = arc_center(b.0, c.0, beta)
        .ok_or("stanowisko leży na prostej przez drugi i trzeci punkt")?;
    let mut p = reflect(b.0, m1, m2)
        .ok_or("stanowisko leży na okręgu przez trzy punkty (okrąg niebezpieczny)")?;
    let r1 = (a.0.x - m1.x).hypot(a.0.y - m1.y);
    let danger = (m1.x - m2.x).hypot(m1.y - m2.y) / r1;
    if danger < 1e-6 {
        return Err("stanowisko leży na okręgu przez trzy punkty (okrąg niebezpieczny)".into());
    }

    let orient_of = |p: Point2| -> Result<f64, String> {
        let (s, c) = obs.iter().try_fold((0.0, 0.0), |(s, c), &(q, r)| {
            let (az, _) =
                azimuth_distance(p, q).ok_or("stanowisko pokrywa się z punktem znanym")?;
            Ok::<_, String>((s + (az - r).sin(), c + (az - r).cos()))
        })?;
        Ok(s.atan2(c))
    };

    let mut o = orient_of(p)?;
    let n = obs.len();
    let mut cov = None;
    if n > 3 {
        // Gauss-Newton: v = az(P, Q) − r − o, niewiadome dx, dy, do.
        for _ in 0..10 {
            let mut nm = [[0.0f64; 3]; 3];
            let mut rhs = [0.0f64; 3];
            for &(q, r) in obs {
                let (dx, dy) = (q.x - p.x, q.y - p.y);
                let d2 = dx * dx + dy * dy;
                let (az, _) =
                    azimuth_distance(p, q).ok_or("stanowisko pokrywa się z punktem znanym")?;
                let row = [dy / d2, -dx / d2, -1.0];
                let l = signed(r + o - az);
                for i in 0..3 {
                    rhs[i] += row[i] * l;
                    for j in 0..3 {
                        nm[i][j] += row[i] * row[j];
                    }
                }
            }
            let inv = invert3(nm).ok_or("układ równań wcięcia jest osobliwy")?;
            let dlt: Vec<f64> = (0..3)
                .map(|i| (0..3).map(|j| inv[i][j] * rhs[j]).sum())
                .collect();
            p = Point2::new(p.x + dlt[0], p.y + dlt[1]);
            o += dlt[2];
            cov = Some(inv);
            if dlt[0].abs() < 1e-7 && dlt[1].abs() < 1e-7 {
                break;
            }
        }
    }
    let o = o.rem_euclid(TAU);
    let residuals: Vec<f64> = obs
        .iter()
        .map(|&(q, r)| signed(azimuth_distance(p, q).map_or(0.0, |(az, _)| az) - r - o))
        .collect();
    let (m0, mxy) = match cov {
        Some(inv) if n > 3 => {
            let vv: f64 = residuals.iter().map(|v| v * v).sum();
            let m0 = (vv / (n - 3) as f64).sqrt();
            (
                Some(m0),
                Some((m0 * inv[0][0].sqrt(), m0 * inv[1][1].sqrt())),
            )
        }
        _ => (None, None),
    };
    Ok(ResectionResult {
        point: p,
        orientation: o,
        residuals,
        m0,
        mxy,
        danger,
    })
}

fn invert3(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-30 {
        return None;
    }
    let mut r = [[0.0; 3]; 3];
    for (i, row) in r.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            let (a, b) = ((j + 1) % 3, (j + 2) % 3);
            let (c, d) = ((i + 1) % 3, (i + 2) % 3);
            *v = (m[a][c] * m[b][d] - m[a][d] * m[b][c]) / det;
        }
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pikieta_core::grad_to_rad;

    /// Odczyty kierunków ze stanowiska `p` przy stałej orientacji `o` (z ewentualnym błędem).
    fn readings(p: Point2, pts: &[Point2], o: f64) -> Vec<(Point2, f64)> {
        pts.iter()
            .map(|&q| (q, (azimuth_distance(p, q).unwrap().0 - o).rem_euclid(TAU)))
            .collect()
    }

    #[test]
    fn three_points_inside_and_outside_triangle() {
        let known = [
            Point2::new(5_001_000.0, 7_500_000.0),
            Point2::new(5_000_000.0, 7_501_200.0),
            Point2::new(4_999_200.0, 7_499_500.0),
        ];
        for p in [
            Point2::new(5_000_100.0, 7_500_150.0), // w trójkącie
            Point2::new(5_002_300.0, 7_502_100.0), // poza trójkątem
            Point2::new(4_998_000.0, 7_501_800.0),
        ] {
            for o in [0.0, 1.3, 5.9] {
                let r = resection(&readings(p, &known, o)).unwrap();
                assert!(
                    (r.point.x - p.x).abs() < 1e-6 && (r.point.y - p.y).abs() < 1e-6,
                    "{p:?} {o}"
                );
                assert!(signed(r.orientation - o).abs() < 1e-9);
                assert!(r.m0.is_none());
            }
        }
    }

    #[test]
    fn danger_circle_is_detected() {
        // Stanowisko na okręgu przez trzy punkty: rozwiązanie nieoznaczone.
        let c = Point2::new(0.0, 0.0);
        let on = |t: f64| Point2::new(c.x + 100.0 * t.cos(), c.y + 100.0 * t.sin());
        let known = [on(0.3), on(1.9), on(3.6)];
        let p = on(5.0);
        let res = resection(&readings(p, &known, 0.7));
        assert!(res.is_err() || res.unwrap().danger < 1e-3);
    }

    #[test]
    fn four_points_least_squares_with_errors() {
        let p = Point2::new(5_000_000.0, 7_500_000.0);
        let known = [
            Point2::new(5_000_800.0, 7_500_100.0),
            Point2::new(5_000_050.0, 7_500_900.0),
            Point2::new(4_999_300.0, 7_500_200.0),
            Point2::new(4_999_900.0, 7_499_200.0),
        ];
        let mut obs = readings(p, &known, 2.0);
        // Błędy kierunków ±10cc.
        for (i, e) in [10.0, -10.0, 10.0, -10.0].iter().enumerate() {
            obs[i].1 += grad_to_rad(e / 10000.0);
        }
        let r = resection(&obs).unwrap();
        assert!((r.point.x - p.x).abs() < 0.02 && (r.point.y - p.y).abs() < 0.02);
        let m0 = r.m0.unwrap();
        assert!(m0 > 0.0 && m0 < grad_to_rad(0.003));
        let (mx, my) = r.mxy.unwrap();
        assert!(mx > 0.0 && my > 0.0 && mx < 0.05);
        // Suma odchyłek bliska zeru (stała orientacji jest niewiadomą).
        assert!(r.residuals.iter().sum::<f64>().abs() < 1e-9);
    }

    #[test]
    fn needs_three_points() {
        assert!(resection(&[(Point2::new(0.0, 0.0), 0.0), (Point2::new(1.0, 0.0), 1.0)]).is_err());
    }
}
