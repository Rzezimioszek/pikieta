//! Wiązania WebAssembly: cienka warstwa nad crate'ami rdzenia.

use pikieta_calc::LineOffset;
use pikieta_core::{grad_to_rad, rad_to_grad, Point2};
use pikieta_crs::Crs;
use pikieta_io::{ColumnOrder, PointRecord};
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

fn points_from_flat(coords: &[f64]) -> Vec<Point2> {
    coords
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| Point2::new(c[0], c[1]))
        .collect()
}

/// Metoda biegunowa: punkt ze stanowiska po azymucie (grady) i długości. Zwraca `[x, y]`.
#[wasm_bindgen]
pub fn polar(xs: f64, ys: f64, azimuth_grad: f64, distance: f64) -> Vec<f64> {
    let p = pikieta_calc::polar(Point2::new(xs, ys), grad_to_rad(azimuth_grad), distance);
    vec![p.x, p.y]
}

/// Domiary punktu P względem linii A → B. Zwraca `[miara bieżąca, domiar]`
/// (domiar dodatni po prawej) albo `undefined`, gdy A i B się pokrywają.
#[wasm_bindgen]
pub fn line_offset(xa: f64, ya: f64, xb: f64, yb: f64, xp: f64, yp: f64) -> Option<Vec<f64>> {
    let m = pikieta_calc::line_offset(
        Point2::new(xa, ya),
        Point2::new(xb, yb),
        Point2::new(xp, yp),
    )?;
    Some(vec![m.along, m.offset])
}

/// Punkt o zadanych domiarach względem linii A → B. Zwraca `[x, y]`.
#[wasm_bindgen]
pub fn point_from_offset(
    xa: f64,
    ya: f64,
    xb: f64,
    yb: f64,
    along: f64,
    offset: f64,
) -> Option<Vec<f64>> {
    let p = pikieta_calc::point_from_offset(
        Point2::new(xa, ya),
        Point2::new(xb, yb),
        LineOffset { along, offset },
    )?;
    Some(vec![p.x, p.y])
}

/// Przecięcie prostych A → B i C → D. `coords` to `[xa, ya, xb, yb, xc, yc, xd, yd]`.
/// Zwraca `[x, y]` albo `undefined` dla prostych równoległych.
#[wasm_bindgen]
pub fn line_intersection(coords: &[f64]) -> Option<Vec<f64>> {
    let [a, b, c, d] = points_from_flat(coords).try_into().ok()?;
    let p = pikieta_calc::line_intersection(a, b, c, d)?;
    Some(vec![p.x, p.y])
}

/// Wcięcie liniowe. Zwraca `[x lewy, y lewy, x prawy, y prawy]` względem kierunku A → B
/// albo `undefined`, gdy okręgi się nie przecinają.
#[wasm_bindgen]
pub fn distance_intersection(
    xa: f64,
    ya: f64,
    ra: f64,
    xb: f64,
    yb: f64,
    rb: f64,
) -> Option<Vec<f64>> {
    let (l, r) =
        pikieta_calc::distance_intersection(Point2::new(xa, ya), ra, Point2::new(xb, yb), rb)?;
    Some(vec![l.x, l.y, r.x, r.y])
}

/// Pole wieloboku. `coords` to `[x1, y1, x2, y2, ...]`. Zwraca `[pole ze współrzędnych,
/// pole na elipsoidzie]` albo `undefined` dla nieobsługiwanego układu.
#[wasm_bindgen]
pub fn polygon_area(epsg: u32, coords: &[f64]) -> Option<Vec<f64>> {
    let points = points_from_flat(coords);
    let crs = Crs::from_epsg(epsg)?;
    Some(vec![
        pikieta_calc::area(&points),
        crs.ellipsoidal_area(&points),
    ])
}

/// Wynik odczytu wykazu współrzędnych.
#[wasm_bindgen]
pub struct ParsedPoints {
    ids: Vec<String>,
    coords: Vec<f64>,
    codes: Vec<String>,
    errors: Vec<String>,
}

#[wasm_bindgen]
impl ParsedPoints {
    #[wasm_bindgen(getter)]
    pub fn ids(&self) -> Vec<String> {
        self.ids.clone()
    }

    /// Trójki `[x, y, h]`; brak wysokości to `NaN`.
    #[wasm_bindgen(getter)]
    pub fn coords(&self) -> Vec<f64> {
        self.coords.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn codes(&self) -> Vec<String> {
        self.codes.clone()
    }

    /// Komunikaty o wierszach, których nie udało się odczytać.
    #[wasm_bindgen(getter)]
    pub fn errors(&self) -> Vec<String> {
        self.errors.clone()
    }
}

fn column_order(yx: bool) -> ColumnOrder {
    if yx {
        ColumnOrder::NrYX
    } else {
        ColumnOrder::NrXY
    }
}

/// Odczytuje tekstowy wykaz współrzędnych. `yx` oznacza kolejność numer, Y, X.
#[wasm_bindgen]
pub fn parse_points(text: &str, yx: bool) -> ParsedPoints {
    let parsed = pikieta_io::parse_points(text, column_order(yx));
    ParsedPoints {
        coords: parsed
            .points
            .iter()
            .flat_map(|p| [p.x, p.y, p.h.unwrap_or(f64::NAN)])
            .collect(),
        codes: parsed.points.iter().map(|p| p.code.clone()).collect(),
        ids: parsed.points.into_iter().map(|p| p.id).collect(),
        errors: parsed
            .errors
            .into_iter()
            .map(|e| format!("wiersz {}: {}", e.line, e.message))
            .collect(),
    }
}

/// Zapisuje wykaz współrzędnych. `coords` to trójki `[x, y, h]`, brak wysokości to `NaN`.
#[wasm_bindgen]
pub fn write_points(
    ids: Vec<String>,
    coords: &[f64],
    codes: Vec<String>,
    yx: bool,
    decimals: usize,
) -> String {
    let points: Vec<PointRecord> = ids
        .into_iter()
        .zip(coords.as_chunks::<3>().0)
        .zip(codes)
        .map(|((id, c), code)| PointRecord {
            id,
            x: c[0],
            y: c[1],
            h: (!c[2].is_nan()).then_some(c[2]),
            code,
        })
        .collect();
    pikieta_io::write_points(&points, column_order(yx), decimals)
}

/// Zaokrągla regułą „do parzystej” i zwraca zapis z kropką dziesiętną.
#[wasm_bindgen]
pub fn fmt(value: f64, decimals: usize) -> String {
    pikieta_core::round_half_even(value, decimals)
}

/// Składa raport z danych w JSON. `format` to `txt` albo `html`.
#[wasm_bindgen]
pub fn render_report(json: &str, format: &str) -> Result<String, JsError> {
    let report = pikieta_report::from_json(json).map_err(|e| JsError::new(&e))?;
    let generator = format!("Pikieta {}", env!("CARGO_PKG_VERSION"));
    match format {
        "txt" => Ok(pikieta_report::to_text(&report)),
        "html" => Ok(pikieta_report::to_html(&report, &generator)),
        _ => Err(JsError::new("nieznany format raportu")),
    }
}

// --- GML ---

/// Wczytany dokument GML; drzewo XML zostaje w pamięci WASM, do JS trafiają widoki.
#[wasm_bindgen]
pub struct GmlDoc(pikieta_gml::GmlDocument);

#[wasm_bindgen]
impl GmlDoc {
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str) -> Result<GmlDoc, JsError> {
        pikieta_gml::GmlDocument::parse(text)
            .map(GmlDoc)
            .map_err(|e| JsError::new(&format!("Błąd XML w {e}")))
    }

    /// Lekki indeks obiektów (JSON).
    pub fn index_json(&self) -> String {
        self.0.index_json()
    }

    /// Geometria do mapy jako bufor binarny, w układzie `target_epsg` (0 = bez przeliczania).
    pub fn render_buffer(&self, target_epsg: u32) -> Vec<u8> {
        self.0
            .render_buffer((target_epsg != 0).then_some(target_epsg))
    }

    /// Teksty kartograficzne (PrezentacjaGraficzna) w układzie `target_epsg` (0 = bez przeliczania).
    pub fn texts_json(&self, target_epsg: u32) -> String {
        self.0.texts_json((target_epsg != 0).then_some(target_epsg))
    }

    pub fn class_table_json(&self, class: &str) -> String {
        self.0.class_table_json(class)
    }

    pub fn properties_json(&self, feature: usize) -> String {
        self.0.properties_json(feature)
    }

    pub fn issues_json(&self) -> String {
        self.0.issues_json()
    }

    /// Zmienia atrybut; zwraca nową etykietę obiektu.
    pub fn set_property(
        &mut self,
        feature: usize,
        key: &str,
        value: &str,
    ) -> Result<String, JsError> {
        self.0
            .set_property(feature, key, value)
            .map_err(|e| JsError::new(&e))?;
        Ok(self.0.features[feature].label.clone())
    }

    /// Dokument jako tekst; bez zmian identyczny z wczytanym.
    pub fn to_xml(&self) -> String {
        self.0.to_xml()
    }
}

/// Zapisuje punkty jako GML 3.2. `coords` to trójki `[x, y, h]`, brak wysokości to `NaN`.
#[wasm_bindgen]
pub fn points_to_gml(
    ids: Vec<String>,
    coords: &[f64],
    codes: Vec<String>,
    epsg: u32,
    decimals: usize,
) -> String {
    let points: Vec<pikieta_gml::PointOut> = ids
        .iter()
        .zip(coords.as_chunks::<3>().0)
        .zip(&codes)
        .map(|((id, c), code)| pikieta_gml::PointOut {
            id,
            x: c[0],
            y: c[1],
            h: (!c[2].is_nan()).then_some(c[2]),
            code,
        })
        .collect();
    pikieta_gml::points_to_gml(&points, epsg, decimals)
}

/// Ciąg poligonowy z wyrównaniem przybliżonym.
///
/// `kind`: 0 dwustronnie nawiązany, 1 zamknięty, 2 wiszący. `known` to `[x, y]` kolejno: nawiązanie
/// początkowe, punkt początkowy, punkt końcowy, nawiązanie końcowe (dwa ostatnie tylko dla
/// dwustronnie nawiązanego; inaczej NaN). Kąty w gradach. Zwraca JSON z punktami, azymutami
/// i poprawkami (kąty w gradach) albo błąd z opisem.
#[wasm_bindgen]
pub fn traverse(
    kind: u8,
    known: &[f64],
    angles_grad: &[f64],
    distances: &[f64],
    right_angles: bool,
) -> Result<String, JsError> {
    use pikieta_calc::TraverseKind;
    let at = |i: usize| -> Option<Point2> {
        let (x, y) = (*known.get(2 * i)?, *known.get(2 * i + 1)?);
        (x.is_finite() && y.is_finite()).then(|| Point2::new(x, y))
    };
    let kind = match kind {
        0 => TraverseKind::Connected,
        1 => TraverseKind::Closed,
        2 => TraverseKind::Open,
        _ => return Err(JsError::new("nieznany rodzaj ciągu")),
    };
    let t = pikieta_calc::Traverse {
        kind,
        back: at(0).ok_or_else(|| JsError::new("brak punktu nawiązania"))?,
        start: at(1).ok_or_else(|| JsError::new("brak punktu początkowego"))?,
        end: at(2),
        forward: at(3),
        angles: angles_grad.iter().map(|a| grad_to_rad(*a)).collect(),
        distances: distances.to_vec(),
        right_angles,
    };
    let r = pikieta_calc::traverse(&t).map_err(|e| JsError::new(&e))?;
    let grads = |v: &[f64]| v.iter().map(|a| rad_to_grad(*a)).collect::<Vec<_>>();
    Ok(serde_json::json!({
        "points": r.points.iter().flat_map(|p| [p.x, p.y]).collect::<Vec<_>>(),
        "azimuths": grads(&r.azimuths),
        "angleCorrections": grads(&r.angle_corrections),
        "dxCorrections": r.dx_corrections,
        "dyCorrections": r.dy_corrections,
        "angularMisclosure": r.angular_misclosure.map(rad_to_grad),
        "fx": r.misclosure.map(|m| m.0),
        "fy": r.misclosure.map(|m| m.1),
        "length": r.length,
    })
    .to_string())
}

/// Orientacja stanowiska. `refs` to trójki `[x, y, kierunek_g]` nawiązań. Zwraca
/// `[stała_g, odchyłka1_g, długość1, odchyłka2_g, długość2, ...]` albo błąd.
#[wasm_bindgen]
pub fn tachy_orientation(xs: f64, ys: f64, refs: &[f64]) -> Result<Vec<f64>, JsError> {
    let list: Vec<(Point2, f64)> = refs
        .as_chunks::<3>()
        .0
        .iter()
        .map(|r| (Point2::new(r[0], r[1]), grad_to_rad(r[2])))
        .collect();
    let o = pikieta_calc::orientation(Point2::new(xs, ys), &list).map_err(|e| JsError::new(&e))?;
    let mut out = vec![rad_to_grad(o.constant)];
    for (r, d) in o.residuals.iter().zip(&o.distances) {
        out.extend([rad_to_grad(*r), *d]);
    }
    Ok(out)
}

/// Pikieta tachimetryczna. Brak wysokości stanowiska lub kąta zenitalnego jako NaN.
/// Zwraca `[x, y, azymut_g, odl_pozioma, odl_zredukowana, przewyższenie, wysokość]` (NaN, gdy brak).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn tachy_shot(
    xs: f64,
    ys: f64,
    station_h: f64,
    instrument: f64,
    constant_grad: f64,
    hz_grad: f64,
    zenith_grad: f64,
    distance: f64,
    target: f64,
    curvature: bool,
    to_ellipsoid: bool,
    scale: f64,
) -> Result<Vec<f64>, JsError> {
    let opt = |v: f64| v.is_finite().then_some(v);
    let st = pikieta_calc::Station {
        at: Point2::new(xs, ys),
        height: opt(station_h),
        instrument,
    };
    let m = pikieta_calc::Shot {
        hz: grad_to_rad(hz_grad),
        zenith: opt(zenith_grad).map(grad_to_rad),
        distance,
        target,
    };
    let o = pikieta_calc::ShotOptions {
        curvature,
        to_ellipsoid,
        scale,
    };
    let r = pikieta_calc::shot(&st, grad_to_rad(constant_grad), &m, &o)
        .map_err(|e| JsError::new(&e))?;
    Ok(vec![
        r.point.x,
        r.point.y,
        rad_to_grad(r.azimuth),
        r.horizontal,
        r.reduced,
        r.dh.unwrap_or(f64::NAN),
        r.height.unwrap_or(f64::NAN),
    ])
}

/// Elementy tyczenia: stanowisko, nawiązanie z odczytem [g] i punkt tyczony.
/// Zwraca `[azymut_g, odczyt_g, kąt_g, długość_w_układzie]` albo `undefined`, gdy punkty się pokrywają.
#[wasm_bindgen]
pub fn stakeout(coords: &[f64], ref_reading_grad: f64) -> Option<Vec<f64>> {
    let p = points_from_flat(coords);
    let [st, reference, target] = p.as_slice() else {
        return None;
    };
    let e = pikieta_calc::stakeout(*st, *reference, grad_to_rad(ref_reading_grad), *target)?;
    Some(vec![
        rad_to_grad(e.azimuth),
        rad_to_grad(e.reading),
        rad_to_grad(e.angle),
        e.grid,
    ])
}

/// Długość w układzie na długość do odmierzenia w terenie (wysokość NaN: bez redukcji na elipsoidę).
#[wasm_bindgen]
pub fn grid_to_terrain(grid: f64, scale: f64, height: f64) -> f64 {
    pikieta_calc::grid_to_terrain(grid, scale, height.is_finite().then_some(height))
}

/// Wcięcie wstecz. `obs` to trójki `[x, y, odczyt_g]` punktów znanych (co najmniej trzy).
/// Zwraca `[x, y, stała_g, m0_cc, mx, my, wskaźnik_okręgu, odchyłka1_cc, odchyłka2_cc, ...]`
/// (m0, mx, my jako NaN bez obserwacji nadliczbowych) albo błąd.
#[wasm_bindgen]
pub fn resection(obs: &[f64]) -> Result<Vec<f64>, JsError> {
    let list: Vec<(Point2, f64)> = obs
        .as_chunks::<3>()
        .0
        .iter()
        .map(|r| (Point2::new(r[0], r[1]), grad_to_rad(r[2])))
        .collect();
    let r = pikieta_calc::resection(&list).map_err(|e| JsError::new(&e))?;
    let cc = |rad: f64| rad_to_grad(rad) * 10_000.0;
    let (mx, my) = r.mxy.unwrap_or((f64::NAN, f64::NAN));
    let mut out = vec![
        r.point.x,
        r.point.y,
        rad_to_grad(r.orientation),
        r.m0.map_or(f64::NAN, cc),
        mx,
        my,
        r.danger,
    ];
    out.extend(r.residuals.iter().map(|v| cc(*v)));
    Ok(out)
}

/// Wcięcie kątowe w przód: punkty A, B (`[xa, ya, xb, yb]`), kąty α i β [g], strona względem A→B.
/// Zwraca `[x, y]` albo `undefined`, gdy kąty nie wyznaczają punktu.
#[wasm_bindgen]
pub fn angle_intersection(
    coords: &[f64],
    alpha_grad: f64,
    beta_grad: f64,
    left: bool,
) -> Option<Vec<f64>> {
    let p = points_from_flat(coords);
    let [a, b] = p.as_slice() else { return None };
    pikieta_calc::angle_intersection(
        *a,
        *b,
        grad_to_rad(alpha_grad),
        grad_to_rad(beta_grad),
        left,
    )
    .map(|q| vec![q.x, q.y])
}

/// Wcięcie z wyrównaniem. `obs` to szóstki `[rodzaj, x1, y1, x2, y2, wartość]`: rodzaj 0 to długość
/// od punktu 1 [m] (punkt 2 pomijany), 1 to kąt [g] na punkcie 1 od kierunku na punkt 2.
/// Błędy średnie: kąta [cc], długości [mm]. Zwraca JSON z punktem, poprawkami (cc albo mm),
/// m₀, mx, my i listą rozwiązań ścisłych.
#[wasm_bindgen]
pub fn intersection_lsq(
    obs: &[f64],
    sigma_cc: f64,
    sigma_mm: f64,
    pick: usize,
) -> Result<String, JsError> {
    use pikieta_calc::Observation;
    let list: Vec<Observation> = obs
        .as_chunks::<6>()
        .0
        .iter()
        .map(|o| {
            let (p1, p2) = (Point2::new(o[1], o[2]), Point2::new(o[3], o[4]));
            if o[0] == 0.0 {
                Observation::Distance {
                    from: p1,
                    distance: o[5],
                }
            } else {
                Observation::Angle {
                    at: p1,
                    reference: p2,
                    angle: grad_to_rad(o[5]),
                }
            }
        })
        .collect();
    let r = pikieta_calc::intersection(
        &list,
        grad_to_rad(sigma_cc / 10_000.0),
        sigma_mm / 1000.0,
        pick,
    )
    .map_err(|e| JsError::new(&e))?;
    let res: Vec<f64> = list
        .iter()
        .zip(&r.residuals)
        .map(|(o, v)| match o {
            Observation::Distance { .. } => v * 1000.0,
            Observation::Angle { .. } => rad_to_grad(*v) * 10_000.0,
        })
        .collect();
    Ok(serde_json::json!({
        "x": r.point.x,
        "y": r.point.y,
        "residuals": res,
        "m0": r.m0,
        "mx": r.mxy.0,
        "my": r.mxy.1,
        "solutions": r.solutions.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>(),
    })
    .to_string())
}

fn flat_points(pts: &[Point2]) -> Vec<f64> {
    pts.iter().flat_map(|p| [p.x, p.y]).collect()
}

/// Przecięcie prostej A→B z okręgiem (`[xa, ya, xb, yb, xc, yc]`, promień). Zwraca `[x1, y1, x2, y2]`.
#[wasm_bindgen]
pub fn line_circle(coords: &[f64], r: f64) -> Vec<f64> {
    match points_from_flat(coords).as_slice() {
        [a, b, c] => flat_points(&pikieta_calc::line_circle(*a, *b, *c, r)),
        _ => vec![],
    }
}

/// Przecięcie prostej równoległej do A→B (odsunięcie w prawo dodatnie) z prostą C→D.
#[wasm_bindgen]
pub fn parallel_intersection(coords: &[f64], offset: f64) -> Option<Vec<f64>> {
    match points_from_flat(coords).as_slice() {
        [a, b, c, d] => {
            pikieta_calc::parallel_intersection(*a, *b, offset, *c, *d).map(|p| vec![p.x, p.y])
        }
        _ => None,
    }
}

/// Podział odcinka A→B: na `n` części albo (gdy `step` > 0) co `step` metrów.
#[wasm_bindgen]
pub fn divide_segment(coords: &[f64], n: usize, step: f64) -> Vec<f64> {
    match points_from_flat(coords).as_slice() {
        [a, b] if step > 0.0 => flat_points(&pikieta_calc::divide_every(*a, *b, step)),
        [a, b] => flat_points(&pikieta_calc::divide(*a, *b, n)),
        _ => vec![],
    }
}

/// Spodek prostopadłej z P na prostą A→B (`[xa, ya, xb, yb, xp, yp]`).
#[wasm_bindgen]
pub fn perpendicular_foot(coords: &[f64]) -> Option<Vec<f64>> {
    match points_from_flat(coords).as_slice() {
        [a, b, p] => pikieta_calc::foot(*a, *b, *p).map(|q| vec![q.x, q.y]),
        _ => None,
    }
}
