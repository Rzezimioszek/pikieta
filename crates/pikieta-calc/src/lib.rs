//! Obliczenia geodezyjne na płaszczyźnie.
//!
//! Azymuty są w radianach, liczone od północy zgodnie z ruchem wskazówek zegara.

use pikieta_core::Point2;
use std::f64::consts::TAU;

mod intersection;
mod resection;
pub use intersection::{intersection, IntersectionResult, Observation};
mod stakeout;
pub use resection::{resection, ResectionResult};
mod tachymetry;
pub use stakeout::{grid_to_terrain, stakeout, StakeoutElements};
mod traverse;
pub use tachymetry::{orientation, shot, Orientation, Shot, ShotOptions, ShotResult, Station};
pub use traverse::{traverse, Traverse, TraverseKind, TraverseResult};

/// Poniżej tej wartości sinusa kąta między kierunkami proste uznajemy za równoległe.
const PARALLEL_EPS: f64 = 1e-12;

/// Azymut (zakres `[0, 2π)`) i długość odcinka `a → b`.
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

/// Punkt wyznaczony metodą biegunową: ze stanowiska, po azymucie i długości.
pub fn polar(station: Point2, azimuth: f64, distance: f64) -> Point2 {
    Point2::new(
        station.x + distance * azimuth.cos(),
        station.y + distance * azimuth.sin(),
    )
}

/// Miary ortogonalne punktu względem linii pomiarowej.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineOffset {
    /// Miara bieżąca: odległość rzutu punktu od początku linii, dodatnia w stronę końca.
    pub along: f64,
    /// Domiar: odległość od linii, dodatnia po prawej stronie, patrząc od początku do końca.
    pub offset: f64,
}

/// Wektor jednostkowy kierunku `a → b` albo `None` dla punktów pokrywających się.
fn direction(a: Point2, b: Point2) -> Option<(f64, f64)> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let d = dx.hypot(dy);
    (d > 0.0).then(|| (dx / d, dy / d))
}

/// Rzutuje punkt `p` na linię `a → b` (domiary prostokątne).
pub fn line_offset(a: Point2, b: Point2, p: Point2) -> Option<LineOffset> {
    let (ux, uy) = direction(a, b)?;
    let (dx, dy) = (p.x - a.x, p.y - a.y);
    Some(LineOffset {
        along: dx * ux + dy * uy,
        offset: dy * ux - dx * uy,
    })
}

/// Punkt o zadanych miarach ortogonalnych względem linii `a → b`.
pub fn point_from_offset(a: Point2, b: Point2, m: LineOffset) -> Option<Point2> {
    let (ux, uy) = direction(a, b)?;
    Some(Point2::new(
        a.x + m.along * ux - m.offset * uy,
        a.y + m.along * uy + m.offset * ux,
    ))
}

/// Przecięcie dwóch prostych zadanych punktem i azymutem (wcięcie kątowe w przód
/// po przeliczeniu kątów na azymuty). `None` dla prostych równoległych.
pub fn azimuth_intersection(a: Point2, az_a: f64, b: Point2, az_b: f64) -> Option<Point2> {
    let (uax, uay) = (az_a.cos(), az_a.sin());
    let (ubx, uby) = (az_b.cos(), az_b.sin());
    let denom = uax * uby - uay * ubx;
    if denom.abs() < PARALLEL_EPS {
        return None;
    }
    let t = ((b.x - a.x) * uby - (b.y - a.y) * ubx) / denom;
    Some(Point2::new(a.x + t * uax, a.y + t * uay))
}

/// Wcięcie kątowe w przód: punkt z kątów wewnętrznych trójkąta `alpha` (przy `a`, między kierunkami
/// na `b` i na punkt) i `beta` (przy `b`, między kierunkami na `a` i na punkt). `left` wybiera stronę
/// prostej `a → b`, po której leży punkt. `None` dla kątów, których proste się nie przecinają.
pub fn angle_intersection(
    a: Point2,
    b: Point2,
    alpha: f64,
    beta: f64,
    left: bool,
) -> Option<Point2> {
    if alpha <= 0.0 || beta <= 0.0 || alpha + beta >= std::f64::consts::PI {
        return None;
    }
    let (az_ab, _) = azimuth_distance(a, b)?;
    let az_ba = (az_ab + std::f64::consts::PI).rem_euclid(TAU);
    // Po lewej stronie A→B kierunek z A obraca się przeciwnie do ruchu wskazówek zegara.
    let s = if left { -1.0 } else { 1.0 };
    azimuth_intersection(
        a,
        (az_ab + s * alpha).rem_euclid(TAU),
        b,
        (az_ba - s * beta).rem_euclid(TAU),
    )
}

/// Przecięcie prostych `a → b` i `c → d`. `None` dla prostych równoległych
/// albo gdy któraś para punktów się pokrywa.
pub fn line_intersection(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<Point2> {
    let (az_ab, _) = azimuth_distance(a, b)?;
    let (az_cd, _) = azimuth_distance(c, d)?;
    azimuth_intersection(a, az_ab, c, az_cd)
}

/// Wcięcie liniowe: punkty odległe o `ra` od `a` i o `rb` od `b`.
///
/// Zwraca parę `(lewy, prawy)` względem kierunku `a → b`. `None`, gdy okręgi się
/// nie przecinają albo `a` i `b` się pokrywają.
pub fn distance_intersection(a: Point2, ra: f64, b: Point2, rb: f64) -> Option<(Point2, Point2)> {
    let (_, d) = azimuth_distance(a, b)?;
    if d > ra + rb || d < (ra - rb).abs() {
        return None;
    }
    let along = (ra * ra - rb * rb + d * d) / (2.0 * d);
    let offset = (ra * ra - along * along).max(0.0).sqrt();
    let left = point_from_offset(
        a,
        b,
        LineOffset {
            along,
            offset: -offset,
        },
    )?;
    let right = point_from_offset(a, b, LineOffset { along, offset })?;
    Some((left, right))
}

/// Pole wieloboku ze współrzędnych (wzór Gaussa), ze znakiem: dodatnie dla
/// wierzchołków podanych zgodnie z ruchem wskazówek zegara na mapie.
///
/// Współrzędne są redukowane do pierwszego wierzchołka, żeby duże wartości
/// (np. 7 500 000 m w PL-2000) nie zjadały precyzji.
pub fn signed_area(points: &[Point2]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let o = points[0];
    let mut doubled = 0.0;
    for i in 0..n {
        let prev = points[(i + n - 1) % n];
        let next = points[(i + 1) % n];
        doubled += (points[i].x - o.x) * (next.y - prev.y);
    }
    doubled / 2.0
}

/// Pole wieloboku ze współrzędnych, niezależnie od kierunku obejścia.
pub fn area(points: &[Point2]) -> f64 {
    signed_area(points).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pikieta_core::{grad_to_rad, rad_to_grad};

    #[test]
    fn angle_intersection_both_sides() {
        // A na zachodzie, B na wschodzie, kąty 50g: punkt 50 m na północ (lewa strona A→B) albo na południe.
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(0.0, 100.0));
        let g50 = grad_to_rad(50.0);
        let l = angle_intersection(a, b, g50, g50, true).unwrap();
        assert!((l.x - 50.0).abs() < 1e-9 && (l.y - 50.0).abs() < 1e-9);
        let r = angle_intersection(a, b, g50, g50, false).unwrap();
        assert!((r.x + 50.0).abs() < 1e-9 && (r.y - 50.0).abs() < 1e-9);
        // Wzór z kotangensami: xP = (xA ctg β + xB ctg α + (yB − yA)) / (ctg α + ctg β).
        let (alpha, beta) = (grad_to_rad(63.1234), grad_to_rad(48.765));
        let (ca, cb) = (1.0 / alpha.tan(), 1.0 / beta.tan());
        let p = angle_intersection(a, b, alpha, beta, true).unwrap();
        assert!((p.x - (a.x * cb + b.x * ca + (b.y - a.y)) / (ca + cb)).abs() < 1e-9);
        assert!((p.y - (a.y * cb + b.y * ca - (b.x - a.x)) / (ca + cb)).abs() < 1e-9);
        assert!(angle_intersection(a, b, grad_to_rad(120.0), grad_to_rad(90.0), true).is_none());
    }

    fn az_grad(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
        let (az, d) = azimuth_distance(Point2::new(a.0, a.1), Point2::new(b.0, b.1)).unwrap();
        (rad_to_grad(az), d)
    }

    fn close(p: Point2, x: f64, y: f64) -> bool {
        (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9
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

    #[test]
    fn polar_inverts_azimuth_distance() {
        let a = Point2::new(1000.0, 2000.0);
        let b = Point2::new(1300.0, 2400.0);
        let (az, d) = azimuth_distance(a, b).unwrap();
        assert!(close(polar(a, az, d), b.x, b.y));
        assert!(close(polar(a, grad_to_rad(100.0), 10.0), 1000.0, 2010.0));
    }

    #[test]
    fn offsets_right_is_positive() {
        // Linia na północ: punkt na wschód od niej leży po prawej.
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(100.0, 0.0));
        let m = line_offset(a, b, Point2::new(30.0, 4.0)).unwrap();
        assert!((m.along - 30.0).abs() < 1e-12 && (m.offset - 4.0).abs() < 1e-12);
        let m = line_offset(a, b, Point2::new(-5.0, -2.0)).unwrap();
        assert!((m.along + 5.0).abs() < 1e-12 && (m.offset + 2.0).abs() < 1e-12);
        assert_eq!(line_offset(a, a, b), None);
    }

    #[test]
    fn offsets_round_trip_on_skew_line() {
        let a = Point2::new(5788000.0, 7500000.0);
        let b = Point2::new(5788300.0, 7500400.0);
        let p = Point2::new(5788123.45, 7500210.98);
        let m = line_offset(a, b, p).unwrap();
        let back = point_from_offset(a, b, m).unwrap();
        assert!((back.x - p.x).abs() < 1e-8 && (back.y - p.y).abs() < 1e-8);
    }

    #[test]
    fn intersections() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(0.0, 100.0);
        // Z A azymut 50g, z B azymut 350g: trójkąt równoramienny, wierzchołek (50, 50).
        let p = azimuth_intersection(a, grad_to_rad(50.0), b, grad_to_rad(350.0)).unwrap();
        assert!(close(p, 50.0, 50.0), "{p:?}");
        assert_eq!(azimuth_intersection(a, 1.0, b, 1.0), None);

        let p = line_intersection(a, Point2::new(10.0, 10.0), b, Point2::new(10.0, 90.0)).unwrap();
        assert!(close(p, 50.0, 50.0), "{p:?}");
        let parallel = line_intersection(a, b, Point2::new(5.0, 0.0), Point2::new(5.0, 100.0));
        assert_eq!(parallel, None);
    }

    #[test]
    fn distance_intersection_3_4_5() {
        // Baza na wschód, długość 5; promienie 3 i 4 dają trójkąt prostokątny.
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(0.0, 5.0));
        let (left, right) = distance_intersection(a, 3.0, b, 4.0).unwrap();
        // Patrząc na wschód, lewa strona to północ.
        assert!(close(left, 2.4, 1.8), "{left:?}");
        assert!(close(right, -2.4, 1.8), "{right:?}");
        assert_eq!(distance_intersection(a, 1.0, b, 1.0), None);
        assert_eq!(distance_intersection(a, 10.0, b, 1.0), None);
    }

    #[test]
    fn area_of_polygon() {
        // Kwadrat 1 ha w PL-2000, zgodnie z ruchem wskazówek zegara.
        let (x, y) = (5788000.0, 7500000.0);
        let square = [
            Point2::new(x, y),
            Point2::new(x + 100.0, y),
            Point2::new(x + 100.0, y + 100.0),
            Point2::new(x, y + 100.0),
        ];
        assert!((signed_area(&square) - 10_000.0).abs() < 1e-9);
        let mut reversed = square;
        reversed.reverse();
        assert!((signed_area(&reversed) + 10_000.0).abs() < 1e-9);
        assert!((area(&reversed) - 10_000.0).abs() < 1e-9);
        // Trójkąt 3-4-5.
        let tri = [
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(0.0, 4.0),
        ];
        assert!((area(&tri) - 6.0).abs() < 1e-12);
        assert_eq!(area(&tri[..2]), 0.0);
    }
}
