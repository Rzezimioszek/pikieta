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
