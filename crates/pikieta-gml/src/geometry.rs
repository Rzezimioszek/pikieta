//! Odczyt geometrii GML 3.2 (oraz `gml:coordinates` z GML 2).
//!
//! Współrzędne są zachowywane w kolejności z pliku. Dla układów EPSG:2176–2180
//! kolejność osi to północ, wschód, czyli geodezyjne X, Y.

use crate::xml::{local, Element};
use std::collections::HashSet;
use std::f64::consts::{PI, TAU};

/// Punkt w kolejności osi z pliku.
pub type P = [f64; 2];

#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Point(P),
    Line(Vec<P>),
    /// Pierścień zewnętrzny i wewnętrzne.
    Polygon(Vec<Vec<P>>),
    Multi(Vec<Geometry>),
}

impl Geometry {
    /// Wszystkie wierzchołki, np. do obliczenia zasięgu.
    pub fn vertices(&self) -> Vec<P> {
        match self {
            Geometry::Point(p) => vec![*p],
            Geometry::Line(l) => l.clone(),
            Geometry::Polygon(rings) => rings.concat(),
            Geometry::Multi(parts) => parts.iter().flat_map(Geometry::vertices).collect(),
        }
    }

    /// Pierścienie wszystkich wieloboków.
    pub fn rings(&self) -> Vec<&Vec<P>> {
        match self {
            Geometry::Polygon(rings) => rings.iter().collect(),
            Geometry::Multi(parts) => parts.iter().flat_map(Geometry::rings).collect(),
            _ => Vec::new(),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Geometry::Point(_) => "point",
            Geometry::Line(_) => "line",
            Geometry::Polygon(_) => "polygon",
            Geometry::Multi(_) => "multi",
        }
    }
}

/// Elementy geometrii GML rozpoznawane przez odczyt.
const GEOMETRY_NAMES: &[&str] = &[
    "Point",
    "LineString",
    "LinearRing",
    "Curve",
    "CompositeCurve",
    "OrientableCurve",
    "Ring",
    "Polygon",
    "Surface",
    "PolygonPatch",
    "CompositeSurface",
    "OrientableSurface",
    "MultiPoint",
    "MultiCurve",
    "MultiLineString",
    "MultiSurface",
    "MultiPolygon",
    "MultiGeometry",
];

/// Rozpoznawanie elementów GML po przestrzeni nazw (przedrostki z deklaracji `xmlns`).
#[derive(Debug, Clone, Default)]
pub struct GmlNames {
    prefixes: HashSet<String>,
}

impl GmlNames {
    /// Zbiera przedrostki, pod którymi w dokumencie występuje przestrzeń nazw GML.
    pub fn from_root(root: &Element) -> Self {
        let mut prefixes = HashSet::new();
        collect_prefixes(root, &mut prefixes);
        if prefixes.is_empty() {
            prefixes.insert("gml".to_string());
        }
        Self { prefixes }
    }

    pub fn has_prefix(&self, prefix: &str) -> bool {
        self.prefixes.contains(prefix)
    }

    pub fn is_gml(&self, e: &Element) -> bool {
        self.prefixes.contains(e.prefix())
    }

    pub fn is_geometry(&self, e: &Element) -> bool {
        self.is_gml(e) && GEOMETRY_NAMES.contains(&e.local_name())
    }

    /// Pierwszy element geometrii w poddrzewie (wraz z samym elementem).
    pub fn find_geometry<'a>(&self, e: &'a Element) -> Option<&'a Element> {
        if self.is_geometry(e) {
            return Some(e);
        }
        e.elements().find_map(|c| self.find_geometry(c))
    }
}

fn collect_prefixes(e: &Element, out: &mut HashSet<String>) {
    for a in &e.attrs {
        let is_gml = crate::xml::unescape(&a.raw).starts_with("http://www.opengis.net/gml");
        if is_gml && &*a.name == "xmlns" {
            out.insert(String::new());
        } else if let Some(p) = a.name.strip_prefix("xmlns:") {
            if is_gml {
                out.insert(p.to_string());
            }
        }
    }
    for c in e.elements() {
        collect_prefixes(c, out);
    }
}

/// Błąd odczytu geometrii.
pub type GeoResult<T> = Result<T, String>;

fn dimension(e: &Element, inherited: usize) -> usize {
    e.attr_local("srsDimension")
        .and_then(|d| d.trim().parse().ok())
        .filter(|d| (1..=4).contains(d))
        .unwrap_or(inherited)
}

fn numbers(text: &str) -> GeoResult<Vec<f64>> {
    text.split_ascii_whitespace()
        .map(|t| {
            t.parse::<f64>()
                .map_err(|_| format!("„{t}” nie jest liczbą"))
        })
        .collect()
}

fn to_points(values: &[f64], dim: usize, what: &str) -> GeoResult<Vec<P>> {
    if dim < 2 || !values.len().is_multiple_of(dim) {
        return Err(format!(
            "{what}: {} liczb nie dzieli się na punkty o wymiarze {dim}",
            values.len()
        ));
    }
    Ok(values.chunks(dim).map(|c| [c[0], c[1]]).collect())
}

/// `gml:coordinates` z GML 2: „x,y x,y” z ustawianymi separatorami.
fn coordinates(e: &Element) -> GeoResult<Vec<P>> {
    let cs = e.attr("cs").unwrap_or_else(|| ",".into());
    let ts = e.attr("ts").unwrap_or_else(|| " ".into());
    let dec = e.attr("decimal").unwrap_or_else(|| ".".into());
    let text = e.text();
    let tuples: Vec<&str> = if ts.trim().is_empty() {
        text.split_whitespace().collect()
    } else {
        text.split(ts.as_str())
            .filter(|t| !t.trim().is_empty())
            .collect()
    };
    tuples
        .iter()
        .map(|t| {
            let v: Vec<f64> = t
                .split(cs.as_str())
                .map(|n| n.trim().replace(dec.as_str(), ".").parse::<f64>())
                .collect::<Result<_, _>>()
                .map_err(|_| format!("błędne współrzędne „{}”", t.trim()))?;
            if v.len() < 2 {
                return Err(format!("za mało współrzędnych w „{}”", t.trim()));
            }
            Ok([v[0], v[1]])
        })
        .collect()
}

/// Ciąg punktów elementu liniowego: `posList`, seria `pos`/`pointProperty` albo `coordinates`.
fn point_list(e: &Element, dim: usize) -> GeoResult<Vec<P>> {
    let dim = dimension(e, dim);
    if let Some(pl) = e.child("posList") {
        let d = dimension(pl, dim);
        return to_points(&numbers(&pl.text())?, d, "posList");
    }
    if let Some(c) = e.child("coordinates") {
        return coordinates(c);
    }
    let mut out = Vec::new();
    for c in e.elements() {
        match c.local_name() {
            "pos" => out.push(pos(c)?),
            "pointProperty" | "pointRep" => {
                if let Some(pt) = c.child("Point") {
                    out.push(point(pt)?);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Jeden `gml:pos`: pierwsze dwie współrzędne (trzecia to wysokość).
fn pos(e: &Element) -> GeoResult<P> {
    match numbers(&e.text())?.as_slice() {
        [a, b, ..] => Ok([*a, *b]),
        _ => Err("gml:pos ma mniej niż dwie współrzędne".into()),
    }
}

fn point(e: &Element) -> GeoResult<P> {
    if let Some(c) = e.child("coordinates") {
        return coordinates(c)?
            .first()
            .copied()
            .ok_or_else(|| "puste gml:coordinates".to_string());
    }
    pos(e.child("pos").ok_or("Point bez gml:pos")?)
}

/// Łuk przez trzy punkty zamieniony na łamaną; kroki co najwyżej 2°.
fn arc(p1: P, p2: P, p3: P, full_circle: bool) -> Vec<P> {
    let (ax, ay, bx, by, cx, cy) = (p1[0], p1[1], p2[0], p2[1], p3[0], p3[1]);
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-12 {
        return if full_circle {
            vec![p1, p2, p3]
        } else {
            vec![p1, p3]
        };
    }
    let a2 = ax * ax + ay * ay;
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;
    let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
    let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
    let r = (ax - ux).hypot(ay - uy);
    let ang = |x: f64, y: f64| (y - uy).atan2(x - ux);
    let (t1, t2, t3) = (ang(ax, ay), ang(bx, by), ang(cx, cy));
    // Kierunek wyznacza punkt środkowy: sprawdzamy, czy leży na drodze przeciwnej do zegara.
    let ccw = |from: f64, to: f64| (to - from).rem_euclid(TAU);
    let sweep = if full_circle {
        if ccw(t1, t2) < PI {
            TAU
        } else {
            -TAU
        }
    } else if ccw(t1, t2) <= ccw(t1, t3) {
        ccw(t1, t3)
    } else {
        ccw(t1, t3) - TAU
    };
    let steps = ((sweep.abs() / 2f64.to_radians()).ceil() as usize).max(2);
    let mut out: Vec<P> = (0..=steps)
        .map(|i| {
            let t = t1 + sweep * i as f64 / steps as f64;
            [ux + r * t.cos(), uy + r * t.sin()]
        })
        .collect();
    // Dokładne końce zamiast przybliżonych.
    out[0] = p1;
    let last = out.len() - 1;
    out[last] = if full_circle { p1 } else { p3 };
    out
}

fn append(line: &mut Vec<P>, part: Vec<P>) {
    let skip = usize::from(!line.is_empty() && line.last() == part.first());
    line.extend(part.into_iter().skip(skip));
}

/// Odcinki krzywej `gml:segments`.
fn curve_segments(e: &Element, dim: usize) -> GeoResult<Vec<P>> {
    let segs = e.child("segments").ok_or("Curve bez gml:segments")?;
    let mut out = Vec::new();
    for s in segs.elements() {
        let pts = point_list(s, dim)?;
        match s.local_name() {
            "LineStringSegment" | "GeodesicString" => append(&mut out, pts),
            "Arc" | "ArcString" => {
                if pts.len() < 3 || pts.len().is_multiple_of(2) {
                    return Err(format!(
                        "{}: potrzeba nieparzystej liczby punktów ≥ 3",
                        s.local_name()
                    ));
                }
                for w in pts.windows(3).step_by(2) {
                    append(&mut out, arc(w[0], w[1], w[2], false));
                }
            }
            "Circle" => {
                if pts.len() != 3 {
                    return Err("Circle: potrzeba trzech punktów".into());
                }
                append(&mut out, arc(pts[0], pts[1], pts[2], true));
            }
            other => return Err(format!("nieobsługiwany odcinek krzywej {other}")),
        }
    }
    Ok(out)
}

/// Dowolna geometria liniowa jako łamana.
fn linear(e: &Element, dim: usize) -> GeoResult<Vec<P>> {
    let dim = dimension(e, dim);
    match e.local_name() {
        "LineString" | "LinearRing" => point_list(e, dim),
        "Curve" => curve_segments(e, dim),
        "OrientableCurve" => {
            let base = e
                .child("baseCurve")
                .and_then(|b| b.elements().next())
                .ok_or("OrientableCurve bez baseCurve")?;
            let mut pts = linear(base, dim)?;
            if e.attr("orientation").as_deref() == Some("-") {
                pts.reverse();
            }
            Ok(pts)
        }
        "CompositeCurve" | "Ring" => {
            let mut out = Vec::new();
            for m in e.elements().filter(|m| m.local_name() == "curveMember") {
                let c = m.elements().next().ok_or("pusty curveMember")?;
                append(&mut out, linear(c, dim)?);
            }
            Ok(out)
        }
        other => Err(format!("{other} nie jest geometrią liniową")),
    }
}

fn ring_of(boundary: &Element, dim: usize) -> GeoResult<Vec<P>> {
    let r = boundary
        .elements()
        .next()
        .ok_or_else(|| format!("pusty {}", boundary.local_name()))?;
    linear(r, dim)
}

fn polygon(e: &Element, dim: usize) -> GeoResult<Geometry> {
    let dim = dimension(e, dim);
    let mut rings = Vec::new();
    for b in e.elements() {
        match b.local_name() {
            "exterior" | "outerBoundaryIs" => rings.insert(0, ring_of(b, dim)?),
            "interior" | "innerBoundaryIs" => rings.push(ring_of(b, dim)?),
            _ => {}
        }
    }
    if rings.is_empty() {
        return Err(format!("{} bez pierścienia zewnętrznego", e.local_name()));
    }
    Ok(Geometry::Polygon(rings))
}

fn members(e: &Element, dim: usize, member: &[&str]) -> GeoResult<Vec<Geometry>> {
    let mut out = Vec::new();
    for m in e.elements() {
        if member.contains(&m.local_name()) {
            for g in m.elements() {
                out.push(read(g, dim)?);
            }
        }
    }
    Ok(out)
}

fn simplify(mut parts: Vec<Geometry>) -> Geometry {
    if parts.len() == 1 {
        parts.pop().unwrap()
    } else {
        Geometry::Multi(parts)
    }
}

/// Odczytuje geometrię z elementu GML. `dim` to wymiar współrzędnych przejęty z nadrzędnych elementów.
pub fn read(e: &Element, dim: usize) -> GeoResult<Geometry> {
    let dim = dimension(e, dim);
    match e.local_name() {
        "Point" => Ok(Geometry::Point(point(e)?)),
        "LineString" | "LinearRing" | "Curve" | "OrientableCurve" | "CompositeCurve" | "Ring" => {
            Ok(Geometry::Line(linear(e, dim)?))
        }
        "Polygon" | "PolygonPatch" => polygon(e, dim),
        "Surface" => {
            let patches = e.child("patches").ok_or("Surface bez gml:patches")?;
            let parts = patches
                .elements()
                .map(|p| polygon(p, dim))
                .collect::<GeoResult<_>>()?;
            Ok(simplify(parts))
        }
        "OrientableSurface" => {
            let base = e
                .child("baseSurface")
                .and_then(|b| b.elements().next())
                .ok_or("OrientableSurface bez baseSurface")?;
            read(base, dim)
        }
        "MultiPoint" => Ok(Geometry::Multi(members(
            e,
            dim,
            &["pointMember", "pointMembers"],
        )?)),
        "MultiCurve" | "MultiLineString" => Ok(Geometry::Multi(members(
            e,
            dim,
            &["curveMember", "curveMembers", "lineStringMember"],
        )?)),
        "MultiSurface" | "MultiPolygon" | "CompositeSurface" => Ok(simplify(members(
            e,
            dim,
            &["surfaceMember", "surfaceMembers", "polygonMember"],
        )?)),
        "MultiGeometry" => Ok(Geometry::Multi(members(
            e,
            dim,
            &["geometryMember", "geometryMembers"],
        )?)),
        other => Err(format!("nieobsługiwana geometria {}", local(other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::parse;

    fn geom(xml: &str) -> GeoResult<Geometry> {
        let doc = parse(&format!(
            "<r xmlns:gml=\"http://www.opengis.net/gml/3.2\">{xml}</r>"
        ))
        .unwrap();
        let root = doc.root().unwrap();
        let names = GmlNames::from_root(root);
        read(names.find_geometry(root).unwrap(), 2)
    }

    #[test]
    fn point_and_lines() {
        assert_eq!(
            geom("<gml:Point><gml:pos>5788400.1 7500800.2</gml:pos></gml:Point>").unwrap(),
            Geometry::Point([5788400.1, 7500800.2])
        );
        assert_eq!(
            geom("<gml:LineString srsDimension=\"3\"><gml:posList>1 2 9 3 4 9</gml:posList></gml:LineString>")
                .unwrap(),
            Geometry::Line(vec![[1.0, 2.0], [3.0, 4.0]])
        );
        assert_eq!(
            geom("<gml:LineString><gml:pos>1 2</gml:pos><gml:pos>3 4</gml:pos></gml:LineString>")
                .unwrap(),
            Geometry::Line(vec![[1.0, 2.0], [3.0, 4.0]])
        );
        assert_eq!(
            geom("<gml:LineString><gml:coordinates>1,2 3,4</gml:coordinates></gml:LineString>")
                .unwrap(),
            Geometry::Line(vec![[1.0, 2.0], [3.0, 4.0]])
        );
        assert!(geom("<gml:LineString><gml:posList>1 2 3</gml:posList></gml:LineString>").is_err());
        assert!(geom("<gml:LineString><gml:posList>1 x</gml:posList></gml:LineString>").is_err());
    }

    #[test]
    fn polygon_with_hole_and_surface() {
        let p = geom(
            "<gml:Polygon><gml:exterior><gml:LinearRing><gml:posList>0 0 0 10 10 10 10 0 0 0</gml:posList>\
             </gml:LinearRing></gml:exterior><gml:interior><gml:LinearRing><gml:posList>2 2 2 3 3 3 2 2</gml:posList>\
             </gml:LinearRing></gml:interior></gml:Polygon>",
        )
        .unwrap();
        assert_eq!(p.rings().len(), 2);
        assert_eq!(p.rings()[0].len(), 5);
        let s = geom(
            "<gml:Surface><gml:patches><gml:PolygonPatch><gml:exterior><gml:LinearRing>\
             <gml:posList>0 0 0 1 1 1 0 0</gml:posList></gml:LinearRing></gml:exterior></gml:PolygonPatch>\
             </gml:patches></gml:Surface>",
        )
        .unwrap();
        assert_eq!(s.kind(), "polygon");
    }

    #[test]
    fn arc_is_densified_through_middle_point() {
        // Półokrąg o promieniu 10 wokół (0,0): od (10,0) przez (0,10) do (-10,0).
        let g = geom(
            "<gml:Curve><gml:segments><gml:Arc><gml:posList>10 0 0 10 -10 0</gml:posList></gml:Arc>\
             </gml:segments></gml:Curve>",
        )
        .unwrap();
        let Geometry::Line(pts) = g else { panic!() };
        assert_eq!(pts.first(), Some(&[10.0, 0.0]));
        assert_eq!(pts.last(), Some(&[-10.0, 0.0]));
        assert!(pts.len() >= 90);
        for p in &pts {
            assert!((p[0].hypot(p[1]) - 10.0).abs() < 1e-9);
            assert!(p[1] >= -1e-9, "łuk poszedł złą stroną: {p:?}");
        }
        // Ten sam łuk w drugą stronę (przez (0,-10)).
        let g = geom(
            "<gml:Curve><gml:segments><gml:Arc><gml:posList>10 0 0 -10 -10 0</gml:posList></gml:Arc>\
             </gml:segments></gml:Curve>",
        )
        .unwrap();
        assert!(g.vertices().iter().all(|p| p[1] <= 1e-9));
    }

    #[test]
    fn ring_of_curve_members_and_multisurface() {
        let g = geom(
            "<gml:MultiSurface><gml:surfaceMember><gml:Polygon><gml:exterior><gml:Ring>\
             <gml:curveMember><gml:LineString><gml:posList>0 0 0 10 10 10</gml:posList></gml:LineString></gml:curveMember>\
             <gml:curveMember><gml:Curve><gml:segments><gml:LineStringSegment><gml:posList>10 10 0 0</gml:posList>\
             </gml:LineStringSegment></gml:segments></gml:Curve></gml:curveMember>\
             </gml:Ring></gml:exterior></gml:Polygon></gml:surfaceMember></gml:MultiSurface>",
        )
        .unwrap();
        let rings = g.rings();
        assert_eq!(rings.len(), 1);
        assert_eq!(
            rings[0],
            &vec![[0.0, 0.0], [0.0, 10.0], [10.0, 10.0], [0.0, 0.0]]
        );
    }

    #[test]
    fn non_gml_prefix_is_not_geometry() {
        let doc = parse("<r xmlns:x=\"urn:inne\"><x:Point/></r>").unwrap();
        let root = doc.root().unwrap();
        let names = GmlNames::from_root(root);
        // Brak deklaracji GML: domyślnie rozpoznawany jest przedrostek „gml”.
        assert!(names.find_geometry(root).is_none());
    }
}
