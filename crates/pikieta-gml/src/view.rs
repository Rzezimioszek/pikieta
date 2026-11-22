//! Widoki dokumentu dla interfejsu: wspólne dla WASM i natywnej aplikacji (Tauri).
//!
//! Geometria trafia do mapy jako jeden bufor binarny (bez JSON), atrybuty są
//! liczone na żądanie: dla tabeli jednej klasy albo jednego obiektu.

use crate::geometry::{Geometry, P};
use crate::{epsg_from_srs, GmlDocument, Severity};
use pikieta_core::Point2;
use pikieta_crs::{Crs, TransverseMercator};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Kod rodzaju geometrii w indeksie.
fn geom_code(g: Option<&Geometry>) -> u8 {
    match g {
        None => 0,
        Some(Geometry::Point(_)) => 1,
        Some(Geometry::Line(_)) => 2,
        Some(Geometry::Polygon(_)) => 3,
        Some(Geometry::Multi(_)) => 4,
    }
}

/// Przeliczenie z układu pliku do układu mapy; `None`, gdy układy są te same lub nieznane.
struct Reproject {
    from: TransverseMercator,
    to: TransverseMercator,
}

impl Reproject {
    fn new(from: Option<u32>, to: Option<u32>) -> Option<Self> {
        let (from, to) = (Crs::from_epsg(from?)?, Crs::from_epsg(to?)?);
        (from != to).then(|| Reproject {
            from: from.projection(),
            to: to.projection(),
        })
    }

    fn apply(&self, p: P) -> P {
        let q = self.to.forward(self.from.inverse(Point2::new(p[0], p[1])));
        [q.x, q.y]
    }
}

struct Buffers {
    coords: Vec<f64>,
    ops: Vec<u32>,
}

impl Buffers {
    /// Dopisuje punkt w kolejności osi mapy (wschód, północ) i zwraca jego numer.
    fn push(&mut self, p: P, re: Option<&Reproject>) -> u32 {
        let [x, y] = re.map_or(p, |r| r.apply(p));
        let at = (self.coords.len() / 2) as u32;
        self.coords.push(y);
        self.coords.push(x);
        at
    }

    fn line(&mut self, pts: &[P], re: Option<&Reproject>) {
        let start = (self.coords.len() / 2) as u32;
        for &p in pts {
            self.push(p, re);
        }
        self.ops.extend([start, pts.len() as u32]);
    }

    fn parts(&mut self, g: &Geometry, re: Option<&Reproject>) -> u32 {
        match g {
            Geometry::Point(p) => {
                self.ops.push(1);
                let c = self.push(*p, re);
                self.ops.push(c);
                1
            }
            Geometry::Line(l) => {
                self.ops.push(2);
                self.line(l, re);
                1
            }
            Geometry::Polygon(rings) => {
                self.ops.extend([3, rings.len() as u32]);
                for r in rings {
                    self.line(r, re);
                }
                1
            }
            Geometry::Multi(parts) => parts.iter().map(|p| self.parts(p, re)).sum(),
        }
    }
}

impl GmlDocument {
    /// Geometria wszystkich obiektów jako bufor dla mapy.
    ///
    /// Układ bajtów (little endian): `u32` liczba obiektów, `u32` długość `ops`,
    /// `u32` liczba współrzędnych `f64`, `u32` zero; potem współrzędne `f64`
    /// (pary wschód, północ, w układzie `target_epsg`), potem `ops` jako `u32`.
    ///
    /// `ops` dla każdego obiektu po kolei: liczba części, a dla każdej części
    /// punkt `[1, nr]`, linia `[2, nr, n]` albo wielobok `[3, r, (nr, n) × r]`,
    /// gdzie `nr` to numer pierwszej pary współrzędnych.
    pub fn render_buffer(&self, target_epsg: Option<u32>) -> Vec<u8> {
        let mut b = Buffers {
            coords: Vec::new(),
            ops: Vec::with_capacity(self.features.len() * 8),
        };
        let mut cache: HashMap<Option<u32>, Option<Reproject>> = HashMap::new();
        for f in &self.features {
            let Some(g) = &f.geometry else {
                b.ops.push(0);
                continue;
            };
            let src = f.srs.as_deref().and_then(epsg_from_srs);
            let re = cache
                .entry(src)
                .or_insert_with(|| Reproject::new(src, target_epsg))
                .as_ref();
            let at = b.ops.len();
            b.ops.push(0);
            let n = b.parts(g, re);
            b.ops[at] = n;
        }
        let mut out = Vec::with_capacity(16 + b.coords.len() * 8 + b.ops.len() * 4);
        for v in [
            self.features.len() as u32,
            b.ops.len() as u32,
            b.coords.len() as u32,
            0,
        ] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for c in &b.coords {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for o in &b.ops {
            out.extend_from_slice(&o.to_le_bytes());
        }
        out
    }

    /// Najczęstszy kod EPSG geometrii.
    pub fn epsg(&self) -> Option<u32> {
        self.srs().as_deref().and_then(epsg_from_srs)
    }

    /// Lekki indeks obiektów: klasy, etykiety, identyfikatory i rodzaje geometrii.
    pub fn index_json(&self) -> String {
        let mut classes: Vec<&str> = Vec::new();
        let mut class_of: HashMap<&str, usize> = HashMap::new();
        let cls: Vec<usize> = self
            .features
            .iter()
            .map(|f| {
                *class_of.entry(f.class.as_str()).or_insert_with(|| {
                    classes.push(f.class.as_str());
                    classes.len() - 1
                })
            })
            .collect();
        // Kody znaków umownych jako słownik i numer na obiekt.
        let mut codes: Vec<&str> = Vec::new();
        let mut code_ix: HashMap<&str, usize> = HashMap::new();
        let code: Vec<usize> = self
            .features
            .iter()
            .map(|f| {
                *code_ix.entry(f.code.as_str()).or_insert_with(|| {
                    codes.push(f.code.as_str());
                    codes.len() - 1
                })
            })
            .collect();
        let errors: serde_json::Map<String, Value> = self
            .features
            .iter()
            .enumerate()
            .filter_map(|(i, f)| Some((i.to_string(), json!(f.geometry_error.as_ref()?))))
            .collect();
        json!({
            "classes": classes,
            "cls": cls,
            "label": self.features.iter().map(|f| f.label.as_str()).collect::<Vec<_>>(),
            "id": self.features.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            "geom": self.features.iter().map(|f| geom_code(f.geometry.as_ref())).collect::<Vec<_>>(),
            "variant": self.features.iter().map(|f| f.variant).collect::<Vec<_>>(),
            "codes": codes,
            "code": code,
            "epsg": self.epsg(),
            "errors": errors,
        })
        .to_string()
    }

    /// Tabela atrybutów jednej klasy: kolumny w kolejności pojawiania się i wiersze wartości.
    pub fn class_table_json(&self, class: &str) -> String {
        let mut keys: Vec<String> = Vec::new();
        let mut editable: Vec<bool> = Vec::new();
        let mut col: HashMap<String, usize> = HashMap::new();
        let mut rows: Vec<(usize, Vec<(usize, String)>)> = Vec::new();
        for (i, f) in self.features.iter().enumerate() {
            if f.class != class {
                continue;
            }
            let mut cells = Vec::new();
            for p in self.properties(i) {
                let c = *col.entry(p.key.clone()).or_insert_with(|| {
                    keys.push(p.key.clone());
                    editable.push(false);
                    keys.len() - 1
                });
                editable[c] |= p.editable;
                cells.push((c, p.value));
            }
            rows.push((i, cells));
        }
        let rows: Vec<Value> = rows
            .into_iter()
            .map(|(i, cells)| {
                let mut values = vec![String::new(); keys.len()];
                for (c, v) in cells {
                    values[c] = v;
                }
                json!([i, values])
            })
            .collect();
        json!({ "keys": keys, "editable": editable, "rows": rows }).to_string()
    }

    /// Atrybuty jednego obiektu: `[nazwa, wartość, edytowalny]`.
    pub fn properties_json(&self, feature: usize) -> String {
        let list: Vec<Value> = self
            .properties(feature)
            .into_iter()
            .map(|p| json!([p.key, p.value, p.editable]))
            .collect();
        Value::Array(list).to_string()
    }

    pub fn issues_json(&self) -> String {
        let list: Vec<Value> = self
            .issues
            .iter()
            .map(|i| {
                let severity = match i.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                };
                json!({ "severity": severity, "feature": i.feature, "message": i.message })
            })
            .collect();
        Value::Array(list).to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::GmlDocument;

    const EGIB: &str = include_str!("../tests/data/egib-przyklad.gml");

    fn decode(buf: &[u8]) -> (u32, Vec<f64>, Vec<u32>) {
        let u = |i: usize| u32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
        let (n, n_ops, n_coords) = (u(0), u(4) as usize, u(8) as usize);
        let coords = (0..n_coords)
            .map(|k| f64::from_le_bytes(buf[16 + k * 8..24 + k * 8].try_into().unwrap()))
            .collect();
        let base = 16 + n_coords * 8;
        let ops = (0..n_ops).map(|k| u(base + k * 4)).collect();
        (n, coords, ops)
    }

    #[test]
    fn render_buffer_layout() {
        let d = GmlDocument::parse(EGIB).unwrap();
        let (n, coords, ops) = decode(&d.render_buffer(Some(2178)));
        assert_eq!(n as usize, d.features.len());
        // Pierwszy obiekt: obręb (MultiSurface z jednym wielobokiem, 5 wierzchołków).
        assert_eq!(&ops[..5], &[1, 3, 1, 0, 5]);
        // Kolejność osi mapy: wschód, północ.
        assert_eq!(&coords[..2], &[7500700.0, 5788300.0]);
        let pg1 = d.features.iter().position(|f| f.id == "TEST.PG.1").unwrap();
        let mut at = 0;
        for _ in 0..pg1 {
            // Przeskocz obiekty przed punktem: wszystkie mają jedną część.
            let parts = ops[at];
            at += 1;
            for _ in 0..parts {
                at += match ops[at] {
                    1 => 2,
                    2 => 3,
                    3 => 2 + 2 * ops[at + 1] as usize,
                    _ => unreachable!(),
                };
            }
        }
        assert_eq!(&ops[at..at + 2], &[1, 1]);
        let c = ops[at + 2] as usize;
        assert_eq!(&coords[2 * c..2 * c + 2], &[7500800.0, 5788400.0]);
    }

    #[test]
    fn render_buffer_reprojects() {
        let d = GmlDocument::parse(EGIB).unwrap();
        let (_, coords, _) = decode(&d.render_buffer(Some(2180)));
        // Narożnik obrębu w PL-1992 (sprawdzone osobno transformacją pikieta-crs).
        let p = pikieta_crs::transform(
            pikieta_core::Point2::new(5788300.0, 7500700.0),
            pikieta_crs::Crs::Pl2000 { zone: 7 },
            pikieta_crs::Crs::Pl1992,
        );
        assert_eq!(&coords[..2], &[p.y, p.x]);
    }

    #[test]
    fn index_and_tables() {
        let d = GmlDocument::parse(EGIB).unwrap();
        let idx: serde_json::Value = serde_json::from_str(&d.index_json()).unwrap();
        assert_eq!(idx["classes"][0], "EGB_ObrebEwidencyjny");
        assert_eq!(idx["epsg"], 2178);
        assert_eq!(idx["label"][1], "12/1");
        assert_eq!(idx["geom"][1], 3);
        let t: serde_json::Value =
            serde_json::from_str(&d.class_table_json("EGB_DzialkaEwidencyjna")).unwrap();
        let keys: Vec<&str> = t["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap())
            .collect();
        let kw = keys.iter().position(|k| *k == "numerKW").unwrap();
        assert_eq!(t["rows"].as_array().unwrap().len(), 2);
        assert_eq!(t["rows"][0][1][kw], "TE1S/00012345/6");
        assert_eq!(t["editable"][kw], true);
        let pg = keys
            .iter()
            .position(|k| *k == "punktGranicyDzialki")
            .unwrap();
        assert_eq!(t["editable"][pg], false);
        let props: serde_json::Value = serde_json::from_str(&d.properties_json(1)).unwrap();
        assert!(props
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p[0] == "idDzialki"));
    }
}
