//! Podział działki na części o zadanych polach (prostymi równoległymi do boku albo z punktu)
//! i rozrzucenie odchyłki pola na użytki.

use crate::area;
use pikieta_core::Point2;

/// Wynik podziału: linie podziału i pola kolejnych części (ostatnia to reszta działki).
#[derive(Debug, Clone, PartialEq)]
pub struct Division {
    /// Punkty każdej linii podziału na granicy działki, w kolejności wzdłuż linii. Dla podziału
    /// z punktu: `[punkt początkowy, punkt na granicy]`.
    pub cuts: Vec<Vec<Point2>>,
    /// Pola części w kolejności podziału; ostatnie to pozostałość.
    pub areas: Vec<f64>,
    /// Czy któraś linia podziału wychodzi poza działkę (działka wklęsła): wynik wymaga sprawdzenia.
    pub outside: bool,
}

fn cross(o: Point2, a: Point2, b: Point2) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Część wieloboku, w której `h(p) <= c` (algorytm Sutherlanda–Hodgmana dla jednej półpłaszczyzny;
/// działa także dla wieloboków wklęsłych w sensie pola).
fn clip(poly: &[Point2], h: &dyn Fn(Point2) -> f64, c: f64) -> Vec<Point2> {
    let n = poly.len();
    let mut out = Vec::with_capacity(n + 2);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let (hp, hq) = (h(p) - c, h(q) - c);
        if hp <= 0.0 {
            out.push(p);
        }
        if (hp < 0.0 && hq > 0.0) || (hp > 0.0 && hq < 0.0) {
            let t = hp / (hp - hq);
            out.push(Point2::new(p.x + t * (q.x - p.x), p.y + t * (q.y - p.y)));
        }
    }
    out
}

/// Pierwiastek funkcji rosnącej `f` na `[lo, hi]` metodą bisekcji.
fn bisect(f: impl Fn(f64) -> f64, mut lo: f64, mut hi: f64) -> f64 {
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-10 {
            break;
        }
    }
    0.5 * (lo + hi)
}

fn check(poly: &[Point2], parts: &[f64]) -> Result<f64, String> {
    if poly.len() < 3 {
        return Err("Działka musi mieć co najmniej trzy wierzchołki.".into());
    }
    let total = area(poly);
    if total <= 0.0 {
        return Err("Działka ma zerowe pole.".into());
    }
    if parts.iter().any(|a| a.is_nan() || *a <= 0.0) {
        return Err("Pola części muszą być dodatnie.".into());
    }
    if parts.iter().sum::<f64>() >= total {
        return Err("Suma pól części nie jest mniejsza od pola działki.".into());
    }
    Ok(total)
}

fn remainder(total: f64, parts: &[f64]) -> Vec<f64> {
    let mut areas = parts.to_vec();
    areas.push(total - parts.iter().sum::<f64>());
    areas
}

/// Podział prostymi równoległymi do prostej `a → b`, licząc części od strony działki najbliższej
/// tej prostej (zwykle `a`, `b` to bok działki).
pub fn divide_parallel(
    poly: &[Point2],
    a: Point2,
    b: Point2,
    parts: &[f64],
) -> Result<Division, String> {
    let total = check(poly, parts)?;
    let len = (b.x - a.x).hypot(b.y - a.y);
    if len < 1e-9 {
        return Err("Punkty wyznaczające kierunek się pokrywają.".into());
    }
    let s = |p: Point2| cross(a, b, p) / len;
    let (lo, hi) = poly.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(s(*p)), hi.max(s(*p)))
    });
    // Część zaczyna się od tej strony działki, która jest bliżej prostej a → b.
    let sign = if lo.abs() <= hi.abs() { 1.0 } else { -1.0 };
    let h = move |p: Point2| sign * s(p);
    let (h0, h1) = if sign > 0.0 { (lo, hi) } else { (-hi, -lo) };

    let mut cuts = Vec::new();
    let mut acc = 0.0;
    let mut outside = false;
    for part in parts {
        acc += part;
        let c = bisect(|c| area(&clip(poly, &h, c)) - acc, h0, h1);
        let line = crossings(poly, &h, c, a, b);
        outside |= line.len() > 2;
        cuts.push(line);
    }
    Ok(Division {
        cuts,
        areas: remainder(total, parts),
        outside,
    })
}

/// Punkty przecięcia prostej `h = c` z granicą, uporządkowane wzdłuż kierunku `a → b`.
fn crossings(
    poly: &[Point2],
    h: &dyn Fn(Point2) -> f64,
    c: f64,
    a: Point2,
    b: Point2,
) -> Vec<Point2> {
    let n = poly.len();
    let mut pts: Vec<Point2> = Vec::new();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let (hp, hq) = (h(p) - c, h(q) - c);
        if hp.abs() < 1e-9 {
            pts.push(p);
        } else if (hp < 0.0 && hq > 0.0) || (hp > 0.0 && hq < 0.0) {
            let t = hp / (hp - hq);
            pts.push(Point2::new(p.x + t * (q.x - p.x), p.y + t * (q.y - p.y)));
        }
    }
    let along = |p: &Point2| (p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y);
    pts.sort_by(|p, q| along(p).total_cmp(&along(q)));
    pts.dedup_by(|p, q| (p.x - q.x).hypot(p.y - q.y) < 1e-6);
    pts
}

/// Podział prostymi wychodzącymi z wierzchołka `start` (wachlarz): kolejne części idą wzdłuż
/// granicy w kolejności wierzchołków.
pub fn divide_from_point(poly: &[Point2], start: usize, parts: &[f64]) -> Result<Division, String> {
    let total = check(poly, parts)?;
    let n = poly.len();
    if start >= n {
        return Err("Punkt początkowy nie jest wierzchołkiem działki.".into());
    }
    // Wierzchołki od punktu początkowego; parametr t to droga po granicy.
    let ring: Vec<Point2> = (0..=n).map(|i| poly[(start + i) % n]).collect();
    let mut dist = vec![0.0];
    for w in ring.windows(2) {
        dist.push(dist.last().unwrap() + (w[1].x - w[0].x).hypot(w[1].y - w[0].y));
    }
    let perimeter = *dist.last().unwrap();
    let at = |t: f64| -> (usize, Point2) {
        let k = dist.partition_point(|d| *d <= t).clamp(1, n);
        let (p, q) = (ring[k - 1], ring[k]);
        let seg = dist[k] - dist[k - 1];
        let u = if seg > 0.0 {
            (t - dist[k - 1]) / seg
        } else {
            0.0
        };
        (k, Point2::new(p.x + u * (q.x - p.x), p.y + u * (q.y - p.y)))
    };
    let fan = |t: f64| {
        let (k, q) = at(t);
        let mut pts: Vec<Point2> = ring[..k].to_vec();
        pts.push(q);
        area(&pts)
    };
    let p0 = ring[0];
    let mut cuts = Vec::new();
    let mut acc = 0.0;
    let mut outside = false;
    for part in parts {
        acc += part;
        let t = bisect(|t| fan(t) - acc, 0.0, perimeter);
        let (_, q) = at(t);
        outside |= leaves(poly, p0, q);
        cuts.push(vec![p0, q]);
    }
    Ok(Division {
        cuts,
        areas: remainder(total, parts),
        outside,
    })
}

/// Czy odcinek `p → q` przecina granicę działki poza swoimi końcami.
fn leaves(poly: &[Point2], p: Point2, q: Point2) -> bool {
    let n = poly.len();
    let eps = 1e-7;
    (0..n).any(|i| {
        let (c, d) = (poly[i], poly[(i + 1) % n]);
        let (d1, d2) = (cross(c, d, p), cross(c, d, q));
        let (d3, d4) = (cross(p, q, c), cross(p, q, d));
        ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps))
            && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))
    })
}

/// Rozrzucenie odchyłki: pola części proporcjonalnie poprawione tak, by ich suma była równa
/// `total`, i zaokrąglone do `unit` (np. 1 m²) z zachowaniem sumy (metoda największych reszt).
pub fn distribute_area(total: f64, parts: &[f64], unit: f64) -> Result<Vec<f64>, String> {
    let sum: f64 = parts.iter().sum();
    if parts.is_empty() || sum <= 0.0 || parts.iter().any(|p| *p < 0.0) {
        return Err("Pola części muszą być nieujemne, a ich suma dodatnia.".into());
    }
    if unit <= 0.0 {
        return Err("Jednostka zaokrąglenia musi być dodatnia.".into());
    }
    let units = (total / unit).round();
    let exact: Vec<f64> = parts.iter().map(|p| p / sum * units).collect();
    let mut out: Vec<f64> = exact.iter().map(|e| e.floor()).collect();
    let short = (units - out.iter().sum::<f64>()).round() as usize;
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by(|&i, &j| (exact[j] - out[j]).total_cmp(&(exact[i] - out[i])));
    for &i in order.iter().take(short) {
        out[i] += 1.0;
    }
    Ok(out.into_iter().map(|u| u * unit).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 100.0),
            Point2::new(50.0, 100.0),
            Point2::new(50.0, 0.0),
        ]
    }

    #[test]
    fn parallel_to_side() {
        // Prostokąt 50 × 100 m; części równoległe do boku x = 0 (od tej strony).
        let r = divide_parallel(
            &square(),
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 100.0),
            &[1000.0, 1500.0],
        )
        .unwrap();
        assert_eq!(r.areas, vec![1000.0, 1500.0, 2500.0]);
        assert!((r.cuts[0][0].x - 10.0).abs() < 1e-6, "{:?}", r.cuts);
        assert!((r.cuts[1][0].x - 25.0).abs() < 1e-6);
        assert_eq!(r.cuts[0].len(), 2);
        assert!(!r.outside);
        // Ta sama prosta z drugiej strony (bok x = 50) daje części od x = 50.
        let s = divide_parallel(
            &square(),
            Point2::new(50.0, 0.0),
            Point2::new(50.0, 100.0),
            &[1000.0],
        )
        .unwrap();
        assert!((s.cuts[0][0].x - 40.0).abs() < 1e-6);
        assert!(divide_parallel(
            &square(),
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 100.0),
            &[6000.0]
        )
        .is_err());
    }

    #[test]
    fn from_vertex() {
        // Z narożnika (0, 0): pierwszy bok leży na prostej z punktu, więc 1250 m² to trójkąt
        // do punktu (25, 100) na drugim boku.
        let r = divide_from_point(&square(), 0, &[1250.0]).unwrap();
        let q = r.cuts[0][1];
        assert!(
            (q.x - 25.0).abs() < 1e-6 && (q.y - 100.0).abs() < 1e-6,
            "{q:?}"
        );
        // Połowa działki: przekątna do (50, 100).
        let r = divide_from_point(&square(), 0, &[2500.0]).unwrap();
        let q = r.cuts[0][1];
        assert!(
            (q.x - 50.0).abs() < 1e-6 && (q.y - 100.0).abs() < 1e-6,
            "{q:?}"
        );
    }

    #[test]
    fn concave_parcel_is_flagged() {
        // Kształt litery U: prosta pozioma przecina granicę w czterech punktach.
        let u = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 30.0),
            Point2::new(20.0, 30.0),
            Point2::new(20.0, 20.0),
            Point2::new(5.0, 20.0),
            Point2::new(5.0, 10.0),
            Point2::new(20.0, 10.0),
            Point2::new(20.0, 0.0),
        ];
        let total = area(&u);
        let r = divide_parallel(
            &u,
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 30.0),
            &[total * 0.6],
        )
        .unwrap();
        assert!(r.outside && r.cuts[0].len() == 4);
    }

    #[test]
    fn area_distribution_keeps_sum() {
        let r = distribute_area(1000.0, &[333.4, 333.3, 340.0], 1.0).unwrap();
        assert_eq!(r.iter().sum::<f64>(), 1000.0);
        assert_eq!(r, vec![331.0, 331.0, 338.0]);
        let ha = distribute_area(5432.0, &[2000.0, 3500.0], 1.0).unwrap();
        assert_eq!(ha, vec![1975.0, 3457.0]);
    }
}
