//! Ciąg poligonowy: obliczenie z wyrównaniem przybliżonym.
//!
//! Kąty są w radianach. Kąt lewy mierzy się na wierzchołku zgodnie z ruchem wskazówek zegara
//! od kierunku wstecz (na poprzedni punkt) do kierunku w przód; leży po lewej stronie ciągu.
//! Kąt prawy liczy się w drugą stronę. Odchyłkę kątową rozkłada się po równo na kąty,
//! a odchyłki przyrostów proporcjonalnie do długości boków.

use crate::azimuth_distance;
use pikieta_core::Point2;
use std::f64::consts::{PI, TAU};

/// Rodzaj ciągu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraverseKind {
    /// Dwustronnie nawiązany: znane punkty początkowy i końcowy oraz kierunki nawiązania na obu końcach.
    Connected,
    /// Zamknięty: wraca do punktu początkowego; kąt nawiązania na początku i kąt zamykający na końcu.
    Closed,
    /// Wiszący (otwarty): bez kontroli, tylko nawiązanie na początku.
    Open,
}

/// Dane ciągu.
#[derive(Debug, Clone, PartialEq)]
pub struct Traverse {
    pub kind: TraverseKind,
    /// Punkt nawiązania na początku (kierunek wstecz z punktu początkowego).
    pub back: Point2,
    /// Punkt początkowy ciągu.
    pub start: Point2,
    /// Punkt końcowy (tylko ciąg dwustronnie nawiązany).
    pub end: Option<Point2>,
    /// Punkt nawiązania na końcu (tylko ciąg dwustronnie nawiązany).
    pub forward: Option<Point2>,
    /// Kąty na kolejnych wierzchołkach, od punktu początkowego.
    pub angles: Vec<f64>,
    /// Długości boków (poziome, w układzie współrzędnych), w kolejności ciągu.
    pub distances: Vec<f64>,
    /// Kąty prawe zamiast lewych.
    pub right_angles: bool,
}

/// Wynik obliczenia.
#[derive(Debug, Clone, PartialEq)]
pub struct TraverseResult {
    /// Wierzchołki od początkowego; dla ciągu zamkniętego bez powtórzenia punktu początkowego,
    /// dla dwustronnie nawiązanego łącznie z końcowym.
    pub points: Vec<Point2>,
    /// Azymuty boków po wyrównaniu.
    pub azimuths: Vec<f64>,
    /// Poprawki kątów (w kolejności `angles`; kąt nawiązania ciągu zamkniętego ma 0).
    pub angle_corrections: Vec<f64>,
    /// Poprawki przyrostów Δx i Δy na bokach.
    pub dx_corrections: Vec<f64>,
    pub dy_corrections: Vec<f64>,
    /// Odchyłka kątowa: wartość obliczona minus wymagana (brak dla ciągu wiszącego).
    pub angular_misclosure: Option<f64>,
    /// Odchyłki przyrostów fx, fy (po wyrównaniu kątów; brak dla ciągu wiszącego).
    pub misclosure: Option<(f64, f64)>,
    /// Długość ciągu (suma boków).
    pub length: f64,
}

impl TraverseResult {
    /// Odchyłka liniowa √(fx² + fy²).
    pub fn linear_misclosure(&self) -> Option<f64> {
        self.misclosure.map(|(fx, fy)| fx.hypot(fy))
    }
}

/// Kąt sprowadzony do zakresu (−π, π].
fn signed(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    if r > PI {
        r - TAU
    } else {
        r
    }
}

/// Oblicza ciąg poligonowy z wyrównaniem przybliżonym.
pub fn traverse(t: &Traverse) -> Result<TraverseResult, String> {
    let legs = t.distances.len();
    if legs == 0 {
        return Err("ciąg musi mieć co najmniej jeden bok".into());
    }
    if t.distances.iter().any(|d| d.is_nan() || *d <= 0.0) {
        return Err("długości boków muszą być dodatnie".into());
    }
    let need_angles = match t.kind {
        TraverseKind::Connected | TraverseKind::Closed => legs + 1,
        TraverseKind::Open => legs,
    };
    if t.angles.len() != need_angles {
        return Err(format!(
            "ciąg o {legs} bokach wymaga {need_angles} kątów, podano {}",
            t.angles.len()
        ));
    }
    if t.kind == TraverseKind::Closed && legs < 3 {
        return Err("ciąg zamknięty wymaga co najmniej trzech boków".into());
    }
    let (start_az, _) =
        azimuth_distance(t.start, t.back).ok_or("punkt nawiązania pokrywa się z początkowym")?;
    let sign = if t.right_angles { -1.0 } else { 1.0 };
    // Azymut boku z poprzedniego azymutu i kąta na wierzchołku.
    let next = |prev: f64, beta: f64| (prev + PI + sign * beta).rem_euclid(TAU);

    // Azymut wstecz z punktu początkowego to azymut „poprzedniego boku” odwrócony.
    let incoming = (start_az + PI).rem_euclid(TAU);
    let mut corrections = vec![0.0; t.angles.len()];
    let angular_misclosure = match t.kind {
        TraverseKind::Open => None,
        TraverseKind::Connected => {
            let (end, fwd) = t.end.zip(t.forward).ok_or(
                "ciąg dwustronnie nawiązany wymaga punktu końcowego i nawiązania końcowego",
            )?;
            let (end_az, _) = azimuth_distance(end, fwd)
                .ok_or("nawiązanie końcowe pokrywa się z punktem końcowym")?;
            let mut az = incoming;
            for &b in &t.angles {
                az = next(az, b);
            }
            let f = signed(az - end_az);
            let v = -sign * f / t.angles.len() as f64;
            corrections.iter_mut().for_each(|c| *c = v);
            Some(f)
        }
        TraverseKind::Closed => {
            // Kąt nawiązania ustala azymut pierwszego boku; kontrolą jest powrót do tego azymutu
            // po kątach na wierzchołkach i kącie zamykającym na punkcie początkowym.
            let first = next(incoming, t.angles[0]);
            let mut az = first;
            for &b in &t.angles[1..] {
                az = next(az, b);
            }
            let f = signed(az - first);
            let v = -sign * f / (t.angles.len() - 1) as f64;
            corrections[1..].iter_mut().for_each(|c| *c = v);
            Some(f)
        }
    };

    // Azymuty boków z kątów poprawionych.
    let mut azimuths = Vec::with_capacity(legs);
    let mut az = incoming;
    for (b, v) in t.angles.iter().zip(&corrections).take(legs) {
        az = next(az, b + v);
        azimuths.push(az);
    }

    let length: f64 = t.distances.iter().sum();
    let mut dx: Vec<f64> = (0..legs)
        .map(|i| t.distances[i] * azimuths[i].cos())
        .collect();
    let mut dy: Vec<f64> = (0..legs)
        .map(|i| t.distances[i] * azimuths[i].sin())
        .collect();
    let target = match t.kind {
        TraverseKind::Connected => t.end,
        TraverseKind::Closed => Some(t.start),
        TraverseKind::Open => None,
    };
    let mut dx_corrections = vec![0.0; legs];
    let mut dy_corrections = vec![0.0; legs];
    let misclosure = target.map(|e| {
        let fx = dx.iter().sum::<f64>() - (e.x - t.start.x);
        let fy = dy.iter().sum::<f64>() - (e.y - t.start.y);
        for i in 0..legs {
            let w = t.distances[i] / length;
            dx_corrections[i] = -fx * w;
            dy_corrections[i] = -fy * w;
            dx[i] += dx_corrections[i];
            dy[i] += dy_corrections[i];
        }
        (fx, fy)
    });

    let mut points = vec![t.start];
    let mut p = t.start;
    for (ddx, ddy) in dx.iter().zip(&dy) {
        p = Point2::new(p.x + ddx, p.y + ddy);
        points.push(p);
    }
    match (t.kind, target) {
        // Ostatni punkt to znany punkt końcowy (początkowy); bierzemy go dokładnie, bez resztek zaokrągleń.
        (TraverseKind::Connected, Some(e)) => *points.last_mut().unwrap() = e,
        (TraverseKind::Closed, _) => {
            points.pop();
        }
        _ => {}
    }

    Ok(TraverseResult {
        points,
        azimuths,
        angle_corrections: corrections,
        dx_corrections,
        dy_corrections,
        angular_misclosure,
        misclosure,
        length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pikieta_core::{grad_to_rad, rad_to_grad};

    fn g(v: f64) -> f64 {
        grad_to_rad(v)
    }

    /// Kąt lewy na wierzchołku `p` od `prev` do `next` (do budowania danych testowych).
    fn left_angle(prev: Point2, p: Point2, next: Point2) -> f64 {
        let (a_back, _) = azimuth_distance(p, prev).unwrap();
        let (a_fwd, _) = azimuth_distance(p, next).unwrap();
        (a_fwd - a_back).rem_euclid(TAU)
    }

    fn dist(a: Point2, b: Point2) -> f64 {
        azimuth_distance(a, b).unwrap().1
    }

    /// Ciąg z prawdziwego kształtu: A, P0, 1, 2, P3, B.
    fn sample() -> (Vec<Point2>, Point2, Point2) {
        let back = Point2::new(5_000_500.0, 7_500_000.0);
        let pts = vec![
            Point2::new(5_000_000.0, 7_500_000.0),
            Point2::new(5_000_100.0, 7_500_150.0),
            Point2::new(5_000_050.0, 7_500_320.0),
            Point2::new(5_000_180.0, 7_500_460.0),
        ];
        let fwd = Point2::new(5_000_600.0, 7_500_900.0);
        (pts, back, fwd)
    }

    fn exact(kind: TraverseKind) -> Traverse {
        let (pts, back, fwd) = sample();
        let n = pts.len();
        let mut angles = vec![left_angle(back, pts[0], pts[1])];
        for i in 1..n - 1 {
            angles.push(left_angle(pts[i - 1], pts[i], pts[i + 1]));
        }
        angles.push(left_angle(pts[n - 2], pts[n - 1], fwd));
        Traverse {
            kind,
            back,
            start: pts[0],
            end: Some(pts[n - 1]),
            forward: Some(fwd),
            angles,
            distances: (0..n - 1).map(|i| dist(pts[i], pts[i + 1])).collect(),
            right_angles: false,
        }
    }

    #[test]
    fn connected_without_errors_reproduces_points() {
        let t = exact(TraverseKind::Connected);
        let r = traverse(&t).unwrap();
        assert!(r.angular_misclosure.unwrap().abs() < 1e-12);
        assert!(r.linear_misclosure().unwrap() < 1e-6);
        let (pts, _, _) = sample();
        for (a, b) in r.points.iter().zip(&pts) {
            assert!((a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6);
        }
    }

    #[test]
    fn connected_distributes_misclosures() {
        let mut t = exact(TraverseKind::Connected);
        // Błąd 0,0040 g na jednym kącie i 5 cm na jednym boku.
        t.angles[1] += g(0.0040);
        t.distances[0] += 0.05;
        let r = traverse(&t).unwrap();
        let f = rad_to_grad(r.angular_misclosure.unwrap());
        assert!((f - 0.0040).abs() < 1e-9, "fα = {f}");
        // Poprawki po równo i z przeciwnym znakiem.
        for c in &r.angle_corrections {
            assert!((rad_to_grad(*c) + 0.0040 / 4.0).abs() < 1e-9);
        }
        // Po wyrównaniu suma przyrostów zgadza się z różnicą współrzędnych końców.
        let (fx, fy) = r.misclosure.unwrap();
        let sx: f64 = r.dx_corrections.iter().sum();
        let sy: f64 = r.dy_corrections.iter().sum();
        assert!((sx + fx).abs() < 1e-9 && (sy + fy).abs() < 1e-9);
        assert!(r.linear_misclosure().unwrap() > 0.04);
        assert_eq!(*r.points.last().unwrap(), t.end.unwrap());
    }

    #[test]
    fn right_angles_give_the_same_points() {
        let left = exact(TraverseKind::Connected);
        let mut right = left.clone();
        right.right_angles = true;
        right.angles = left.angles.iter().map(|a| TAU - a).collect();
        let (a, b) = (traverse(&left).unwrap(), traverse(&right).unwrap());
        for (p, q) in a.points.iter().zip(&b.points) {
            assert!((p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9);
        }
    }

    #[test]
    fn closed_traverse_square() {
        // Kwadrat 100 m: P0 (0,0) → (100,0) → (100,100) → (0,100) → P0, nawiązanie na zachód.
        let p0 = Point2::new(0.0, 0.0);
        let back = Point2::new(0.0, -50.0);
        let corners = [
            p0,
            Point2::new(100.0, 0.0),
            Point2::new(100.0, 100.0),
            Point2::new(0.0, 100.0),
        ];
        let mut angles = vec![left_angle(back, p0, corners[1])];
        for i in 1..4 {
            angles.push(left_angle(corners[i - 1], corners[i], corners[(i + 1) % 4]));
        }
        angles.push(left_angle(corners[3], p0, corners[1]));
        angles[2] += g(0.0030);
        let t = Traverse {
            kind: TraverseKind::Closed,
            back,
            start: p0,
            end: None,
            forward: None,
            angles,
            distances: vec![100.0; 4],
            right_angles: false,
        };
        let r = traverse(&t).unwrap();
        assert!((rad_to_grad(r.angular_misclosure.unwrap()) - 0.0030).abs() < 1e-9);
        assert_eq!(r.angle_corrections[0], 0.0);
        assert_eq!(r.points.len(), 4);
        // Błąd kąta po wyrównaniu przybliżonym zostawia na punktach najwyżej kilka milimetrów.
        assert!((r.points[2].x - 100.0).abs() < 5e-3 && (r.points[2].y - 100.0).abs() < 5e-3);
    }

    #[test]
    fn open_traverse_has_no_check() {
        let mut t = exact(TraverseKind::Open);
        t.angles.pop();
        let r = traverse(&t).unwrap();
        assert!(r.angular_misclosure.is_none() && r.misclosure.is_none());
        let (pts, _, _) = sample();
        let last = r.points.last().unwrap();
        assert!((last.x - pts[3].x).abs() < 1e-6 && (last.y - pts[3].y).abs() < 1e-6);
    }

    #[test]
    fn rejects_wrong_counts() {
        let mut t = exact(TraverseKind::Connected);
        t.angles.pop();
        assert!(traverse(&t).unwrap_err().contains("wymaga 4 kątów"));
    }
}
