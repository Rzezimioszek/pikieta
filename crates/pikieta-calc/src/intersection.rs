//! Wcięcie z wyrównaniem: punkt z dowolnego zestawu kątów i długości do punktów znanych.
//!
//! Obejmuje wcięcia kątowe, liniowe i kombinowane oraz obserwacje nadliczbowe. Wyrównanie metodą
//! najmniejszych kwadratów z wagami z błędów średnich kąta i długości. Punkt startowy z przecięć par
//! obserwacji: wybieramy ten, który po wyrównaniu daje najmniejszą sumę kwadratów poprawek.

use crate::{azimuth_distance, azimuth_intersection, distance_intersection};
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

/// Obserwacja do wyznaczanego punktu P.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Observation {
    /// Długość pozioma od punktu znanego do P.
    Distance { from: Point2, distance: f64 },
    /// Kąt na punkcie znanym `at`, od kierunku na `reference` do kierunku na P (zgodnie z ruchem wskazówek zegara).
    Angle {
        at: Point2,
        reference: Point2,
        angle: f64,
    },
}

impl Observation {
    /// Punkt znany, przy którym obserwacja jest wykonana.
    fn station(&self) -> Point2 {
        match *self {
            Observation::Distance { from, .. } => from,
            Observation::Angle { at, .. } => at,
        }
    }

    /// Wartość wyliczona dla P, pochodne po x, y i błąd średni.
    fn model(&self, p: Point2, sigma_angle: f64, sigma_dist: f64) -> Option<(f64, [f64; 2], f64)> {
        match *self {
            Observation::Distance { from, distance } => {
                let (_, d) = azimuth_distance(from, p)?;
                Some((
                    d - distance,
                    [(p.x - from.x) / d, (p.y - from.y) / d],
                    sigma_dist,
                ))
            }
            Observation::Angle {
                at,
                reference,
                angle,
            } => {
                let (az_p, d) = azimuth_distance(at, p)?;
                let (az_r, _) = azimuth_distance(at, reference)?;
                let d2 = d * d;
                let v = signed(az_p - az_r - angle);
                Some((v, [-(p.y - at.y) / d2, (p.x - at.x) / d2], sigma_angle))
            }
        }
    }
}

/// Wynik wcięcia z wyrównaniem.
#[derive(Debug, Clone, PartialEq)]
pub struct IntersectionResult {
    pub point: Point2,
    /// Poprawki obserwacji: kąty w radianach, długości w metrach (wartość z punktu minus pomiar).
    pub residuals: Vec<f64>,
    /// Błąd średni jednostkowy m₀ (tylko przy obserwacjach nadliczbowych).
    pub m0: Option<f64>,
    /// Błędy średnie współrzędnych mx, my [m] (przy obserwacjach nadliczbowych z m₀, inaczej a priori).
    pub mxy: (f64, f64),
    /// Bez obserwacji nadliczbowych: wszystkie rozwiązania ścisłe (najpierw po lewej stronie prostej
    /// od punktu znanego pierwszej obserwacji do punktu znanego drugiej, potem bliższe); inaczej puste.
    pub solutions: Vec<Point2>,
}

/// Punkty startowe z przecięć par obserwacji (półprosta z kąta, okrąg z długości).
fn candidates(obs: &[Observation]) -> Vec<Point2> {
    enum Geo {
        Ray(Point2, f64),
        Circle(Point2, f64),
    }
    let geo: Vec<Geo> = obs
        .iter()
        .filter_map(|o| match *o {
            Observation::Distance { from, distance } => Some(Geo::Circle(from, distance)),
            Observation::Angle {
                at,
                reference,
                angle,
            } => azimuth_distance(at, reference)
                .map(|(az, _)| Geo::Ray(at, (az + angle).rem_euclid(TAU))),
        })
        .collect();
    let ray_circle = |q: Point2, az: f64, c: Point2, r: f64| -> Vec<Point2> {
        let (ux, uy) = (az.cos(), az.sin());
        let (wx, wy) = (q.x - c.x, q.y - c.y);
        let b = ux * wx + uy * wy;
        let disc = b * b - (wx * wx + wy * wy - r * r);
        if disc < 0.0 {
            return vec![];
        }
        [-b - disc.sqrt(), -b + disc.sqrt()]
            .into_iter()
            .filter(|t| *t > 0.0)
            .map(|t| Point2::new(q.x + t * ux, q.y + t * uy))
            .collect()
    };
    let mut out = Vec::new();
    for i in 0..geo.len() {
        for j in i + 1..geo.len() {
            match (&geo[i], &geo[j]) {
                (Geo::Ray(a, aa), Geo::Ray(b, ab)) => {
                    out.extend(azimuth_intersection(*a, *aa, *b, *ab))
                }
                (Geo::Ray(q, az), Geo::Circle(c, r)) | (Geo::Circle(c, r), Geo::Ray(q, az)) => {
                    out.extend(ray_circle(*q, *az, *c, *r))
                }
                (Geo::Circle(a, ra), Geo::Circle(b, rb)) => {
                    if let Some((l, r)) = distance_intersection(*a, *ra, *b, *rb) {
                        out.extend([l, r]);
                    }
                }
            }
        }
    }
    out
}

/// Wynik wyrównania z jednego punktu startowego.
struct Adjusted {
    point: Point2,
    residuals: Vec<f64>,
    /// Suma ważonych kwadratów poprawek vᵀPv.
    vpv: f64,
    /// Odwrotność macierzy układu równań normalnych.
    inv: [[f64; 2]; 2],
}

/// Wyrównanie z punktu startowego.
fn adjust(obs: &[Observation], start: Point2, sa: f64, sd: f64) -> Option<Adjusted> {
    let mut p = start;
    let mut inv = [[0.0; 2]; 2];
    for _ in 0..20 {
        let (mut n, mut rhs) = ([[0.0f64; 2]; 2], [0.0f64; 2]);
        for o in obs {
            let (v, a, s) = o.model(p, sa, sd)?;
            let w = 1.0 / (s * s);
            for i in 0..2 {
                rhs[i] -= w * a[i] * v;
                for j in 0..2 {
                    n[i][j] += w * a[i] * a[j];
                }
            }
        }
        let det = n[0][0] * n[1][1] - n[0][1] * n[1][0];
        if det.abs() < 1e-30 {
            return None;
        }
        inv = [
            [n[1][1] / det, -n[0][1] / det],
            [-n[1][0] / det, n[0][0] / det],
        ];
        let dx = inv[0][0] * rhs[0] + inv[0][1] * rhs[1];
        let dy = inv[1][0] * rhs[0] + inv[1][1] * rhs[1];
        p = Point2::new(p.x + dx, p.y + dy);
        if dx.abs() < 1e-8 && dy.abs() < 1e-8 {
            break;
        }
    }
    let mut vpv = 0.0;
    let mut res = Vec::with_capacity(obs.len());
    for o in obs {
        let (v, _, s) = o.model(p, sa, sd)?;
        vpv += v * v / (s * s);
        res.push(v);
    }
    Some(Adjusted {
        point: p,
        residuals: res,
        vpv,
        inv,
    })
}

/// Wcięcie z wyrównaniem. `sigma_angle` [rad] i `sigma_dist` [m] to błędy średnie pomiarów (wagi).
/// Bez obserwacji nadliczbowych zwykle są dwa rozwiązania; `pick` wybiera numer z `solutions`.
pub fn intersection(
    obs: &[Observation],
    sigma_angle: f64,
    sigma_dist: f64,
    pick: usize,
) -> Result<IntersectionResult, String> {
    if obs.len() < 2 {
        return Err("potrzeba co najmniej dwóch obserwacji".into());
    }
    let starts = candidates(obs);
    if starts.is_empty() {
        return Err("obserwacje się nie przecinają (sprawdź kąty i długości)".into());
    }
    let n = obs.len();
    let mut found: Vec<Adjusted> = Vec::new();
    for s in starts {
        let Some(adj) = adjust(obs, s, sigma_angle, sigma_dist) else {
            continue;
        };
        let p = adj.point;
        if p.x.is_finite()
            && p.y.is_finite()
            && !found
                .iter()
                .any(|f| (f.point.x - p.x).hypot(f.point.y - p.y) < 1e-4)
        {
            found.push(adj);
        }
    }
    if found.is_empty() {
        return Err("nie udało się wyrównać obserwacji".into());
    }
    // Rozwiązania ścisłe (suma kwadratów poprawek bliska zeru) przy braku obserwacji nadliczbowych.
    let (a, b) = (obs[0].station(), obs[1].station());
    // W układzie geodezyjnym (x północ, y wschód) lewa strona A→B ma iloczyn wektorowy ujemny.
    let right_of = |p: Point2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) > 0.0;
    let near = |p: Point2| (p.x - a.x).hypot(p.y - a.y);
    let exact: Vec<usize> = if n == 2 {
        let mut idx: Vec<usize> = (0..found.len()).filter(|&i| found[i].vpv < 1e-6).collect();
        idx.sort_by(|&i, &j| {
            (right_of(found[i].point), near(found[i].point))
                .partial_cmp(&(right_of(found[j].point), near(found[j].point)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        idx
    } else {
        Vec::new()
    };
    let chosen = if exact.is_empty() {
        (0..found.len())
            .min_by(|&i, &j| {
                found[i]
                    .vpv
                    .partial_cmp(&found[j].vpv)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap()
    } else {
        exact[pick.min(exact.len() - 1)]
    };
    let solutions: Vec<Point2> = exact.iter().map(|&i| found[i].point).collect();
    let Adjusted {
        point,
        residuals,
        inv,
        ..
    } = found.swap_remove(chosen);
    let redundancy = n.saturating_sub(2);
    let m0 = (redundancy > 0).then(|| {
        let vpv: f64 = obs
            .iter()
            .zip(&residuals)
            .map(|(o, v)| {
                let s = match o {
                    Observation::Distance { .. } => sigma_dist,
                    Observation::Angle { .. } => sigma_angle,
                };
                v * v / (s * s)
            })
            .sum();
        (vpv / redundancy as f64).sqrt()
    });
    let k = m0.unwrap_or(1.0);
    Ok(IntersectionResult {
        point,
        residuals,
        m0,
        mxy: (k * inv[0][0].sqrt(), k * inv[1][1].sqrt()),
        solutions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pikieta_core::grad_to_rad;

    fn angle(at: Point2, reference: Point2, p: Point2) -> Observation {
        let a = azimuth_distance(at, p).unwrap().0 - azimuth_distance(at, reference).unwrap().0;
        Observation::Angle {
            at,
            reference,
            angle: a.rem_euclid(TAU),
        }
    }
    fn dist(from: Point2, p: Point2) -> Observation {
        Observation::Distance {
            from,
            distance: azimuth_distance(from, p).unwrap().1,
        }
    }

    const SA: f64 = 10.0 / 10000.0 * PI / 200.0;
    const SD: f64 = 0.005;

    #[test]
    fn combined_angle_and_distance() {
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(0.0, 200.0));
        let p = Point2::new(120.0, 80.0);
        // Kąt przy A od B do P i długość z B.
        // Półprosta z A przecina okrąg wokół B dwa razy: oba rozwiązania są na liście, P jest jednym z nich.
        let obs = [angle(a, b, p), dist(b, p)];
        let r = intersection(&obs, SA, SD, 0).unwrap();
        assert!(r.m0.is_none());
        let k = r
            .solutions
            .iter()
            .position(|q| (q.x - p.x).abs() < 1e-6 && (q.y - p.y).abs() < 1e-6)
            .expect("P wśród rozwiązań");
        let r = intersection(&obs, SA, SD, k).unwrap();
        assert!((r.point.x - p.x).abs() < 1e-6 && (r.point.y - p.y).abs() < 1e-6);
    }

    #[test]
    fn linear_intersection_picks_side() {
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(0.0, 100.0));
        let north = Point2::new(60.0, 40.0);
        let south = Point2::new(-60.0, 40.0);
        let l = intersection(&[dist(a, north), dist(b, north)], SA, SD, 0).unwrap();
        assert_eq!(l.solutions.len(), 2);
        assert!((l.point.x - north.x).abs() < 1e-6, "{:?}", l.point);
        let r = intersection(&[dist(a, south), dist(b, south)], SA, SD, 1).unwrap();
        assert!((r.point.x - south.x).abs() < 1e-6, "{:?}", r.point);
    }

    #[test]
    fn redundant_observations_are_adjusted() {
        let (a, b, c) = (
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 300.0),
            Point2::new(250.0, 400.0),
        );
        let p = Point2::new(180.0, 150.0);
        let mut obs = vec![angle(a, b, p), angle(b, a, p), dist(a, p), dist(c, p)];
        // Błędy: 10cc i 5 mm.
        if let Observation::Angle { angle, .. } = &mut obs[0] {
            *angle += grad_to_rad(0.0010);
        }
        if let Observation::Distance { distance, .. } = &mut obs[3] {
            *distance += 0.005;
        }
        let r = intersection(&obs, SA, SD, 0).unwrap();
        assert!(r.solutions.is_empty());
        assert!((r.point.x - p.x).abs() < 0.01 && (r.point.y - p.y).abs() < 0.01);
        let m0 = r.m0.unwrap();
        assert!(m0 > 0.0 && m0 < 3.0, "m0 = {m0}");
        assert!(r.mxy.0 > 0.0 && r.mxy.0 < 0.02);
    }

    #[test]
    fn impossible_observations() {
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(0.0, 100.0));
        let obs = [
            Observation::Distance {
                from: a,
                distance: 10.0,
            },
            Observation::Distance {
                from: b,
                distance: 10.0,
            },
        ];
        assert!(intersection(&obs, SA, SD, 0).is_err());
    }
}
