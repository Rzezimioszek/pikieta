//! Transformacje płaskie między układami pierwotnym i wtórnym na podstawie punktów dostosowania:
//! Helmerta (przez podobieństwo, 4 parametry) i afiniczna (6 parametrów), z korektą Hausbrandta.

use pikieta_core::Point2;

/// Współrzędne punktu względem środków ciężkości: x, y pierwotne i x, y wtórne.
type Diff = (f64, f64, f64, f64);

/// Rodzaj transformacji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    /// Przez podobieństwo: przesunięcie, obrót i jedna skala; co najmniej 2 punkty.
    Helmert,
    /// Afiniczna: osobne skale i skośność; co najmniej 3 punkty niewspółliniowe.
    Affine,
}

impl TransformKind {
    fn unknowns(self) -> usize {
        match self {
            TransformKind::Helmert => 4,
            TransformKind::Affine => 6,
        }
    }
}

/// Transformacja `X = x0 + a·(x − cx) + b·(y − cy)`, `Y = y0 + c·(x − cx) + d·(y − cy)`,
/// gdzie `(cx, cy)` i `(x0, y0)` to środki ciężkości punktów dostosowania w obu układach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneTransform {
    pub kind: TransformKind,
    pub from_center: Point2,
    pub to_center: Point2,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

impl PlaneTransform {
    pub fn apply(&self, p: Point2) -> Point2 {
        let (dx, dy) = (p.x - self.from_center.x, p.y - self.from_center.y);
        Point2::new(
            self.to_center.x + self.a * dx + self.b * dy,
            self.to_center.y + self.c * dx + self.d * dy,
        )
    }

    /// Skale wzdłuż osi x i y układu pierwotnego (w Helmercie równe).
    pub fn scales(&self) -> (f64, f64) {
        (self.a.hypot(self.c), self.b.hypot(self.d))
    }

    /// Kąty obrotu osi x i y [rad], dodatnie zgodnie z ruchem wskazówek zegara na mapie
    /// (jak przyrost azymutu); w Helmercie równe.
    pub fn rotations(&self) -> (f64, f64) {
        (self.c.atan2(self.a), (-self.b).atan2(self.d))
    }
}

/// Wynik dopasowania transformacji do punktów dostosowania.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformFit {
    pub transform: PlaneTransform,
    /// Odchyłki na punktach dostosowania: współrzędne wtórne minus przetransformowane.
    pub residuals: Vec<Point2>,
    /// Błąd średni transformacji (na współrzędną); `None` bez punktów nadliczbowych.
    pub m0: Option<f64>,
}

impl TransformFit {
    /// Punkt przetransformowany; z korektą Hausbrandta, gdy `hausbrandt`.
    pub fn apply(&self, from: &[Point2], p: Point2, hausbrandt: bool) -> Point2 {
        let q = self.transform.apply(p);
        if !hausbrandt {
            return q;
        }
        let v = hausbrandt_correction(from, &self.residuals, p);
        Point2::new(q.x + v.x, q.y + v.y)
    }
}

/// Dopasowuje transformację metodą najmniejszych kwadratów. `from` i `to` to te same punkty
/// w układzie pierwotnym i wtórnym.
pub fn fit_transform(
    kind: TransformKind,
    from: &[Point2],
    to: &[Point2],
) -> Result<TransformFit, String> {
    let n = from.len();
    if n != to.len() {
        return Err("Liczba punktów w obu układach musi być równa.".into());
    }
    let min = kind.unknowns() / 2;
    if n < min {
        return Err(format!("Potrzeba co najmniej {min} punktów dostosowania."));
    }
    let center = |pts: &[Point2]| {
        let (sx, sy) = pts
            .iter()
            .fold((0.0, 0.0), |(sx, sy), p| (sx + p.x, sy + p.y));
        Point2::new(sx / n as f64, sy / n as f64)
    };
    let (cf, ct) = (center(from), center(to));
    let d: Vec<Diff> = from
        .iter()
        .zip(to)
        .map(|(f, t)| (f.x - cf.x, f.y - cf.y, t.x - ct.x, t.y - ct.y))
        .collect();
    // Skala zbioru punktów, żeby progi nie zależały od jednostek i rozmiaru obiektu.
    let spread: f64 = d.iter().map(|(x, y, _, _)| x * x + y * y).sum();
    if spread < 1e-12 {
        return Err("Punkty dostosowania w układzie pierwotnym się pokrywają.".into());
    }
    let (a, b, c, dd) = match kind {
        TransformKind::Helmert => {
            // Liczby zespolone z = x + iy: w = Σ z'·conj(z) / Σ |z|², X + iY = w·z.
            let (re, im) = d.iter().fold((0.0, 0.0), |(re, im), (x, y, xt, yt)| {
                (re + xt * x + yt * y, im + yt * x - xt * y)
            });
            let (p, q) = (re / spread, im / spread);
            (p, -q, q, p)
        }
        TransformKind::Affine => {
            let (sxx, sxy, syy) = d.iter().fold((0.0, 0.0, 0.0), |(a, b, c), (x, y, _, _)| {
                (a + x * x, b + x * y, c + y * y)
            });
            let det = sxx * syy - sxy * sxy;
            if det.abs() < 1e-9 * spread * spread {
                return Err("Punkty dostosowania leżą na jednej prostej.".into());
            }
            let solve = |f: &dyn Fn(&Diff) -> f64| {
                let (sx, sy) = d
                    .iter()
                    .fold((0.0, 0.0), |(sx, sy), r| (sx + r.0 * f(r), sy + r.1 * f(r)));
                ((syy * sx - sxy * sy) / det, (sxx * sy - sxy * sx) / det)
            };
            let (a, b) = solve(&|r| r.2);
            let (c, dd) = solve(&|r| r.3);
            (a, b, c, dd)
        }
    };
    let transform = PlaneTransform {
        kind,
        from_center: cf,
        to_center: ct,
        a,
        b,
        c,
        d: dd,
    };
    let residuals: Vec<Point2> = from
        .iter()
        .zip(to)
        .map(|(f, t)| {
            let q = transform.apply(*f);
            Point2::new(t.x - q.x, t.y - q.y)
        })
        .collect();
    let redundancy = 2 * n - kind.unknowns();
    let m0 = (redundancy > 0).then(|| {
        let vv: f64 = residuals.iter().map(|v| v.x * v.x + v.y * v.y).sum();
        (vv / redundancy as f64).sqrt()
    });
    Ok(TransformFit {
        transform,
        residuals,
        m0,
    })
}

/// Kontrola punktu dostosowania testem „bez tego punktu”: transformacja dopasowana do pozostałych
/// punktów przenosi go do układu wtórnego, a odległość od jego współrzędnych wtórnych porównuje się
/// z odchyłką dopuszczalną. Gruby błąd punktu nie zawyża wtedy błędu średniego, z którym go
/// porównujemy, ani nie rozkłada się na sąsiednie punkty, jak w zwykłych odchyłkach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointCheck {
    /// Odległość współrzędnych wtórnych od przetransformowanych bez tego punktu [m].
    pub misfit: f64,
    /// Odchyłka dopuszczalna [m]; większa oznacza punkt do sprawdzenia.
    pub limit: f64,
}

impl PointCheck {
    pub fn suspect(&self) -> bool {
        self.misfit > self.limit
    }
}

/// Poziom istotności testu: tyle wynosi szansa oznaczenia dobrego punktu.
pub const CHECK_ALPHA: f64 = 0.01;
/// Najmniejszy błąd średni przyjmowany w teście [m], żeby dane bez szumu (np. wpisane z innego
/// programu z zaokrągleniem) nie dawały progów rzędu mikrometrów.
const M0_FLOOR: f64 = 1e-4;

/// Kontrola każdego punktu dostosowania (kolejność jak w `from`). `None`, gdy bez tego punktu nie
/// zostają co najmniej dwie obserwacje nadliczbowe: wtedy punktu nie da się sprawdzić.
///
/// Statystyka `d² / (2·(1 + h)·m0²)`, gdzie `d` to odległość, `m0` błąd średni dopasowania bez
/// punktu, a `h` wpływ położenia punktu względem pozostałych (dalej od środka, większa
/// niepewność), ma rozkład F(2, r) dla r obserwacji nadliczbowych; jego kwantyl ma postać
/// zamkniętą `r/2·(α^(−2/r) − 1)`.
pub fn check_control(
    kind: TransformKind,
    from: &[Point2],
    to: &[Point2],
) -> Vec<Option<PointCheck>> {
    let n = from.len().min(to.len());
    let u = kind.unknowns();
    (0..n)
        .map(|i| {
            if n < 2 || 2 * (n - 1) < u + 2 {
                return None;
            }
            let rest = |pts: &[Point2]| -> Vec<Point2> {
                pts[..n]
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, p)| *p)
                    .collect()
            };
            let (f, t) = (rest(from), rest(to));
            let fit = fit_transform(kind, &f, &t).ok()?;
            let m0 = fit.m0?.max(M0_FLOOR);
            let q = fit.transform.apply(from[i]);
            let misfit = (to[i].x - q.x).hypot(to[i].y - q.y);
            let h = leverage(kind, &f, from[i]);
            let r = (2 * (n - 1) - u) as f64;
            let f_crit = r / 2.0 * (CHECK_ALPHA.powf(-2.0 / r) - 1.0);
            Some(PointCheck {
                misfit,
                limit: m0 * (2.0 * (1.0 + h) * f_crit).sqrt(),
            })
        })
        .collect()
}

/// Wariancja współrzędnej punktu przetransformowanego, w jednostkach σ²: `1/n` od środka ciężkości
/// plus część od obrotu i skal, rosnąca z odległością od środka.
fn leverage(kind: TransformKind, from: &[Point2], p: Point2) -> f64 {
    let n = from.len() as f64;
    let (cx, cy) = from
        .iter()
        .fold((0.0, 0.0), |(x, y), q| (x + q.x / n, y + q.y / n));
    let (dx, dy) = (p.x - cx, p.y - cy);
    let (sxx, sxy, syy) = from.iter().fold((0.0, 0.0, 0.0), |(a, b, c), q| {
        let (x, y) = (q.x - cx, q.y - cy);
        (a + x * x, b + x * y, c + y * y)
    });
    let part = match kind {
        TransformKind::Helmert => (dx * dx + dy * dy) / (sxx + syy),
        TransformKind::Affine => {
            (syy * dx * dx - 2.0 * sxy * dx * dy + sxx * dy * dy) / (sxx * syy - sxy * sxy)
        }
    };
    1.0 / n + part
}

/// Korekta Hausbrandta: średnia odchyłek punktów dostosowania ważona odwrotnością kwadratu
/// odległości (w układzie pierwotnym). Na punkcie dostosowania równa jego odchyłce, więc
/// punkty dostosowania dostają po transformacji dokładnie współrzędne wtórne.
pub fn hausbrandt_correction(from: &[Point2], residuals: &[Point2], p: Point2) -> Point2 {
    let (mut sw, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for (f, v) in from.iter().zip(residuals) {
        let d2 = (f.x - p.x).powi(2) + (f.y - p.y).powi(2);
        if d2 < 1e-12 {
            return *v;
        }
        let w = 1.0 / d2;
        sw += w;
        sx += w * v.x;
        sy += w * v.y;
    }
    if sw == 0.0 {
        return Point2::new(0.0, 0.0);
    }
    Point2::new(sx / sw, sy / sw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(p: Point2, q: Point2, tol: f64) -> bool {
        (p.x - q.x).abs() < tol && (p.y - q.y).abs() < tol
    }

    /// Obrót o kąt `t` zgodnie z ruchem wskazówek (x północ, y wschód), skala `s`, przesunięcie.
    fn similar(p: Point2, t: f64, s: f64) -> Point2 {
        let (sin, cos) = t.sin_cos();
        Point2::new(
            5000.0 + s * (cos * p.x - sin * p.y),
            7000.0 + s * (sin * p.x + cos * p.y),
        )
    }

    #[test]
    fn helmert_recovers_similarity() {
        let from = [
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(100.0, 100.0),
            Point2::new(0.0, 100.0),
        ];
        let to: Vec<Point2> = from.iter().map(|p| similar(*p, 0.3, 1.0002)).collect();
        let r = fit_transform(TransformKind::Helmert, &from, &to).unwrap();
        let (sx, sy) = r.transform.scales();
        assert!((sx - 1.0002).abs() < 1e-12 && (sy - sx).abs() < 1e-12);
        let (rx, ry) = r.transform.rotations();
        assert!((rx - 0.3).abs() < 1e-12 && (ry - 0.3).abs() < 1e-12);
        assert!(r.m0.unwrap() < 1e-9);
        let p = Point2::new(37.0, -12.0);
        assert!(close(r.transform.apply(p), similar(p, 0.3, 1.0002), 1e-9));
        // Północ (oś x) po obrocie o 0,3 rad w prawo ma dodatni przyrost y: azymut rośnie.
        assert!(
            r.transform.apply(Point2::new(10.0, 0.0)).y
                > r.transform.apply(Point2::new(0.0, 0.0)).y
        );
    }

    #[test]
    fn affine_recovers_general_map_and_needs_three_points() {
        let f = |p: Point2| {
            Point2::new(
                10.0 + 1.01 * p.x + 0.02 * p.y,
                -4.0 - 0.03 * p.x + 0.99 * p.y,
            )
        };
        let from = [
            Point2::new(0.0, 0.0),
            Point2::new(50.0, 10.0),
            Point2::new(20.0, 80.0),
            Point2::new(-30.0, 40.0),
        ];
        let to: Vec<Point2> = from.iter().map(|p| f(*p)).collect();
        let r = fit_transform(TransformKind::Affine, &from, &to).unwrap();
        let p = Point2::new(123.0, 45.0);
        assert!(close(r.transform.apply(p), f(p), 1e-9));
        assert_eq!(r.m0.map(|m| m < 1e-9), Some(true));
        assert!(fit_transform(TransformKind::Affine, &from[..2], &to[..2]).is_err());
        let line = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 2.0),
        ];
        assert!(fit_transform(TransformKind::Affine, &line, &line).is_err());
        // Trzy punkty: brak nadliczbowych.
        assert!(fit_transform(TransformKind::Affine, &from[..3], &to[..3])
            .unwrap()
            .m0
            .is_none());
    }

    #[test]
    fn residuals_and_hausbrandt() {
        let from = [
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(100.0, 100.0),
            Point2::new(0.0, 100.0),
        ];
        let mut to = from.to_vec();
        to[2].x += 0.04; // jeden punkt odstaje o 4 cm
        let r = fit_transform(TransformKind::Helmert, &from, &to).unwrap();
        let sum = r
            .residuals
            .iter()
            .fold((0.0, 0.0), |s, v| (s.0 + v.x, s.1 + v.y));
        assert!(
            sum.0.abs() < 1e-12 && sum.1.abs() < 1e-12,
            "odchyłki sumują się do zera"
        );
        assert!(r.m0.unwrap() > 0.0);
        // Korekta Hausbrandta odtwarza punkty dostosowania dokładnie.
        for (f, t) in from.iter().zip(&to) {
            assert!(close(r.apply(&from, *f, true), *t, 1e-12));
        }
        // Środek kwadratu: średnia z równymi wagami, czyli zero (odchyłki sumują się do zera).
        let mid = Point2::new(50.0, 50.0);
        assert!(close(
            r.apply(&from, mid, true),
            r.transform.apply(mid),
            1e-12
        ));
    }

    /// Typowe zadanie: 5 punktów z szumem kilku milimetrów, jeden przesunięty o 30 cm. Zwykły próg
    /// 3·√2·m0 na odchyłkach nie może go wykryć przy tak małej liczbie punktów; test bez punktu tak.
    #[test]
    fn leave_one_out_finds_gross_error_with_few_points() {
        let from = [
            Point2::new(0.0, 0.0),
            Point2::new(400.0, 30.0),
            Point2::new(380.0, 420.0),
            Point2::new(-20.0, 390.0),
            Point2::new(190.0, 210.0),
        ];
        let noise = [
            (0.003, -0.002),
            (-0.004, 0.001),
            (0.002, 0.004),
            (-0.001, -0.003),
            (0.0, 0.002),
        ];
        let mut to: Vec<Point2> = from
            .iter()
            .zip(noise)
            .map(|(p, (nx, ny))| {
                let q = similar(*p, 0.8, 0.99993);
                Point2::new(q.x + nx, q.y + ny)
            })
            .collect();
        let clean = check_control(TransformKind::Helmert, &from, &to);
        assert!(
            clean.iter().all(|c| c.is_some_and(|c| !c.suspect())),
            "{clean:?}"
        );

        to[2].x += 0.30;
        let fit = fit_transform(TransformKind::Helmert, &from, &to).unwrap();
        let m0 = fit.m0.unwrap();
        assert!(
            fit.residuals
                .iter()
                .all(|v| v.x.hypot(v.y) <= 3.0 * 2f64.sqrt() * m0),
            "stary próg niczego nie oznacza"
        );
        let checks = check_control(TransformKind::Helmert, &from, &to);
        let suspects: Vec<usize> = (0..5).filter(|&i| checks[i].unwrap().suspect()).collect();
        assert_eq!(suspects, vec![2]);
        assert!((checks[2].unwrap().misfit - 0.30).abs() < 0.02);

        // Afiniczna z 5 punktami: bez punktu zostają 2 obserwacje nadliczbowe, więc kontrola jest
        // (słaba); 3 punkty Helmerta: brak kontroli.
        assert!(check_control(TransformKind::Affine, &from, &to)
            .iter()
            .all(|c| c.is_some()));
        assert!(check_control(TransformKind::Helmert, &from[..3], &to[..3])
            .iter()
            .all(|c| c.is_none()));
    }
}
