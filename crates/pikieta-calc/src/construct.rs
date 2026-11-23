//! Konstrukcje geometryczne: przecięcie prostej z okręgiem, prosta równoległa, podział odcinka,
//! rzut punktu na prostą.

use crate::{azimuth_distance, line_intersection, line_offset, point_from_offset, LineOffset};
use pikieta_core::Point2;

/// Punkty przecięcia prostej `a → b` z okręgiem o środku `c` i promieniu `r`, w kolejności wzdłuż
/// prostej od `a`. Pusta lista, gdy prosta nie przecina okręgu albo `a` i `b` się pokrywają.
pub fn line_circle(a: Point2, b: Point2, c: Point2, r: f64) -> Vec<Point2> {
    let Some(m) = line_offset(a, b, c) else {
        return vec![];
    };
    let h2 = r * r - m.offset * m.offset;
    if h2 < 0.0 {
        return vec![];
    }
    let h = h2.sqrt();
    let at = |along: f64| point_from_offset(a, b, LineOffset { along, offset: 0.0 });
    if h < 1e-9 {
        return at(m.along).into_iter().collect();
    }
    [m.along - h, m.along + h]
        .into_iter()
        .filter_map(at)
        .collect()
}

/// Przecięcie prostej równoległej do `a → b`, odsuniętej o `offset` (dodatnie w prawo, patrząc od `a`),
/// z prostą `c → d`. `None` dla prostych równoległych.
pub fn parallel_intersection(
    a: Point2,
    b: Point2,
    offset: f64,
    c: Point2,
    d: Point2,
) -> Option<Point2> {
    let shift = |p: Point2| {
        point_from_offset(
            a,
            b,
            LineOffset {
                along: line_offset(a, b, p)?.along,
                offset,
            },
        )
    };
    let (a2, b2) = (shift(a)?, shift(b)?);
    line_intersection(a2, b2, c, d)
}

/// Punkty dzielące odcinek `a → b` na `n` równych części (bez końców).
pub fn divide(a: Point2, b: Point2, n: usize) -> Vec<Point2> {
    (1..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            Point2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))
        })
        .collect()
}

/// Punkty na odcinku `a → b` co `step` metrów od `a` (bez końców).
pub fn divide_every(a: Point2, b: Point2, step: f64) -> Vec<Point2> {
    let Some((_, len)) = azimuth_distance(a, b) else {
        return vec![];
    };
    if step <= 0.0 {
        return vec![];
    }
    let mut out = Vec::new();
    let mut s = step;
    while s < len - 1e-9 {
        let t = s / len;
        out.push(Point2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)));
        s += step;
    }
    out
}

/// Rzut punktu `p` na prostą `a → b` (spodek prostopadłej).
pub fn foot(a: Point2, b: Point2, p: Point2) -> Option<Point2> {
    let m = line_offset(a, b, p)?;
    point_from_offset(
        a,
        b,
        LineOffset {
            along: m.along,
            offset: 0.0,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(p: Point2, x: f64, y: f64) -> bool {
        (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9
    }

    #[test]
    fn line_and_circle() {
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(0.0, 100.0));
        // Okrąg o środku 3 m na północ od prostej, promień 5 m: przecięcia ±4 m od rzutu środka.
        let r = line_circle(a, b, Point2::new(3.0, 50.0), 5.0);
        assert_eq!(r.len(), 2);
        assert!(close(r[0], 0.0, 46.0) && close(r[1], 0.0, 54.0));
        assert_eq!(line_circle(a, b, Point2::new(5.0, 50.0), 5.0).len(), 1);
        assert!(line_circle(a, b, Point2::new(6.0, 50.0), 5.0).is_empty());
    }

    #[test]
    fn parallel_line_offset_to_the_right() {
        // A→B na wschód; w prawo to na południe. Równoległa 10 m w prawo przecięta z prostą x = const.
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(0.0, 100.0));
        let (c, d) = (Point2::new(50.0, 30.0), Point2::new(-50.0, 30.0));
        let p = parallel_intersection(a, b, 10.0, c, d).unwrap();
        assert!(close(p, -10.0, 30.0), "{p:?}");
        let q = parallel_intersection(a, b, -10.0, c, d).unwrap();
        assert!(close(q, 10.0, 30.0));
    }

    #[test]
    fn division_and_foot() {
        let (a, b) = (Point2::new(0.0, 0.0), Point2::new(100.0, 0.0));
        let d = divide(a, b, 4);
        assert_eq!(d.len(), 3);
        assert!(close(d[1], 50.0, 0.0));
        let e = divide_every(a, b, 30.0);
        assert_eq!(e.len(), 3);
        assert!(close(e[2], 90.0, 0.0));
        assert!(divide_every(a, b, 100.0).is_empty());
        assert!(close(
            foot(a, b, Point2::new(40.0, 7.0)).unwrap(),
            40.0,
            0.0
        ));
    }
}
