//! Odczyt i zapis GML: geometria 3.2 oraz profil EGiB.
//!
//! Dokument jest trzymany jako bezstratne drzewo XML ([`xml`]), a obiekty
//! ([`Feature`]) są jego widokiem: geometria do mapy, atrybuty do tabeli.
//! Zapis bez zmian daje plik identyczny z wczytanym; edycja atrybutu zmienia
//! tylko jego element.

pub mod geometry;
mod view;
pub mod xml;

use geometry::{Geometry, GmlNames, P};
use pikieta_core::round_half_even;
use std::collections::{HashMap, HashSet};
use xml::{Element, XmlError};

/// Atrybut obiektu pokazywany w tabeli.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// Nazwa lokalna, dla zagnieżdżonych ścieżka z kropkami (np. `idIIP.lokalnyId`).
    pub key: String,
    pub value: String,
    /// Prosty element tekstowy, który można zmienić.
    pub editable: bool,
    path: Vec<usize>,
}

/// Obiekt z kolekcji GML.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Nazwa lokalna elementu, np. `EGB_DzialkaEwidencyjna`.
    pub class: String,
    /// `gml:id`.
    pub id: String,
    /// Krótka etykieta do mapy, np. numer działki.
    pub label: String,
    /// Wariant stylu wynikający z atrybutów (bity `VARIANT_*`).
    pub variant: u8,
    /// Kod znaku umownego mapy zasadniczej (Dz.U. 2021 poz. 1385, zał. 4), np. `EGDE`, `SUWP`, `OTKJ`;
    /// pusty, gdy obiekt nie ma znaku w katalogu.
    pub code: String,
    pub geometry: Option<Geometry>,
    pub geometry_error: Option<String>,
    /// `srsName` geometrii.
    pub srs: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Wynik walidacji.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    /// Indeks obiektu w [`GmlDocument::features`].
    pub feature: Option<usize>,
    pub message: String,
}

pub struct GmlDocument {
    doc: xml::Document,
    names: GmlNames,
    /// Ścieżka do elementu obiektu w drzewie (indeksy dzieci od korzenia).
    paths: Vec<Vec<usize>>,
    pub features: Vec<Feature>,
    pub issues: Vec<Issue>,
}

/// Kod EPSG z `srsName` w dowolnym typowym zapisie
/// (`urn:ogc:def:crs:EPSG::2178`, `EPSG:2178`, `http://www.opengis.net/def/crs/EPSG/0/2178`).
pub fn epsg_from_srs(srs: &str) -> Option<u32> {
    let upper = srs.to_ascii_uppercase();
    let at = upper.find("EPSG")?;
    let tail = &upper[at..];
    let digits: String = tail
        .chars()
        .rev()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.chars().rev().collect::<String>().parse().ok()
}

/// Identyfikatory EGiB, z których powstaje etykieta obiektu.
const LABEL_KEYS: &[&str] = &[
    "idDzialki",
    "idPunktu",
    "idBudynku",
    "idLokalu",
    "idObrebu",
    "idJednostkiEwid",
    "idUzytku",
    "idKonturu",
    "numer",
    "nazwa",
    "name",
];

fn is_member(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.ends_with("member") || l.ends_with("members")
}

impl GmlDocument {
    pub fn parse(text: &str) -> Result<Self, XmlError> {
        let doc = xml::parse(text)?;
        let names = doc.root().map(GmlNames::from_root).unwrap_or_default();
        let mut g = GmlDocument {
            doc,
            names,
            paths: Vec::new(),
            features: Vec::new(),
            issues: Vec::new(),
        };
        g.extract();
        g.validate();
        Ok(g)
    }

    /// Dokument jako tekst; bez zmian identyczny z wczytanym.
    pub fn to_xml(&self) -> String {
        self.doc.to_xml()
    }

    /// `srsName` występujący najczęściej w geometriach.
    pub fn srs(&self) -> Option<String> {
        let mut count: HashMap<&str, usize> = HashMap::new();
        for f in &self.features {
            if let Some(s) = &f.srs {
                *count.entry(s).or_default() += 1;
            }
        }
        count
            .into_iter()
            .max_by_key(|(s, n)| (*n, std::cmp::Reverse(*s)))
            .map(|(s, _)| s.to_string())
    }

    fn element(&self, feature: usize) -> Option<&Element> {
        self.doc.root()?.element_at(self.paths.get(feature)?)
    }

    /// Atrybuty obiektu, liczone na żądanie (przy dużych plikach nie trzymamy ich dla wszystkich obiektów).
    pub fn properties(&self, feature: usize) -> Vec<Property> {
        let (Some(el), Some(path)) = (self.element(feature), self.paths.get(feature)) else {
            return Vec::new();
        };
        let mut props = Vec::new();
        for (i, child) in el.children.iter().enumerate() {
            let xml::Node::Element(c) = child else {
                continue;
            };
            if (self.names.is_gml(c) && c.local_name() == "boundedBy")
                || self.names.find_geometry(c).is_some()
            {
                continue;
            }
            let mut cpath = path.clone();
            cpath.push(i);
            properties(c, c.local_name().to_string(), cpath, &mut props);
        }
        merge_duplicates(&mut props);
        props
    }

    /// Wartość jednego atrybutu obiektu.
    pub fn property(&self, feature: usize, key: &str) -> Option<String> {
        self.properties(feature)
            .into_iter()
            .find(|p| p.key == key)
            .map(|p| p.value)
    }

    /// Zmienia wartość prostego atrybutu tekstowego.
    pub fn set_property(&mut self, feature: usize, key: &str, value: &str) -> Result<(), String> {
        if feature >= self.features.len() {
            return Err("nie ma takiego obiektu".into());
        }
        let p = self
            .properties(feature)
            .into_iter()
            .find(|p| p.key == key)
            .ok_or_else(|| format!("obiekt nie ma atrybutu {key}"))?;
        if !p.editable {
            return Err(format!("atrybutu {key} nie można zmieniać"));
        }
        let el = self
            .doc
            .root_mut()
            .and_then(|r| r.element_at_mut(&p.path))
            .ok_or("nie znaleziono elementu atrybutu")?;
        el.set_text(value);
        // Etykieta i wariant stylu zależą od atrybutów, więc liczymy je od nowa.
        let class = self.features[feature].class.clone();
        if let Some(el) = self.element(feature) {
            let d = describe(&class, el);
            let f = &mut self.features[feature];
            f.label = if d.label.is_empty() {
                f.id.clone()
            } else {
                d.label
            };
            f.variant = d.variant;
            f.code = d.code;
        }
        Ok(())
    }

    fn extract(&mut self) {
        let Some(root) = self.doc.root() else { return };
        let names = &self.names;
        let default_srs = root
            .child("boundedBy")
            .and_then(|b| b.child("Envelope"))
            .and_then(|e| e.attr("srsName"))
            .or_else(|| root.attr_local("srsName"));

        let mut found = Vec::new();
        let gml_id = |e: &Element| {
            e.attrs
                .iter()
                .find(|a| xml::local(&a.name) == "id" && names.is_gml_attr(&a.name))
                .map(|a| xml::unescape(&a.raw))
        };
        let root_is_feature = !root.local_name().contains("Collection")
            && gml_id(root).is_some()
            && !names.is_geometry(root);
        if root_is_feature {
            found.push(Vec::new());
        } else {
            collect_features(root, &mut Vec::new(), names, &gml_id, &mut found);
        }

        for path in found {
            let el = root.element_at(&path).expect("ścieżka z przejścia drzewa");
            let mut f = Feature {
                class: el.local_name().to_string(),
                id: gml_id(el).unwrap_or_default(),
                label: String::new(),
                variant: 0,
                code: String::new(),
                geometry: None,
                geometry_error: None,
                srs: None,
            };
            for c in el.elements() {
                if names.is_gml(c) && c.local_name() == "boundedBy" {
                    continue;
                }
                if let Some(g) = names.find_geometry(c) {
                    if f.geometry.is_none() && f.geometry_error.is_none() {
                        match geometry::read(g, 2) {
                            Ok(geom) => f.geometry = Some(geom),
                            Err(e) => f.geometry_error = Some(e),
                        }
                        f.srs = g
                            .attr("srsName")
                            .or_else(|| c.attr("srsName"))
                            .or_else(|| default_srs.clone());
                    }
                }
            }
            let d = describe(&f.class, el);
            (f.label, f.variant, f.code) = (d.label, d.variant, d.code);
            if f.label.is_empty() {
                f.label = f.id.clone();
            }
            self.features.push(f);
            self.paths.push(path);
        }
    }

    fn validate(&mut self) {
        let mut issues = Vec::new();
        let Some(root) = self.doc.root() else { return };
        let names = &self.names;

        // Identyfikatory i odwołania w całym dokumencie.
        let mut ids: HashMap<String, usize> = HashMap::new();
        let mut hrefs: Vec<String> = Vec::new();
        walk(root, &mut |e| {
            for a in &e.attrs {
                if xml::local(&a.name) == "id" && names.is_gml_attr(&a.name) {
                    *ids.entry(xml::unescape(&a.raw)).or_default() += 1;
                }
                if a.name.ends_with(":href") || &*a.name == "href" {
                    hrefs.push(xml::unescape(&a.raw));
                }
            }
        });
        let mut dupes: Vec<_> = ids
            .iter()
            .filter(|(_, n)| **n > 1)
            .map(|(k, _)| k.clone())
            .collect();
        dupes.sort();
        for d in dupes {
            issues.push(Issue {
                severity: Severity::Error,
                feature: self.features.iter().position(|f| f.id == d),
                message: format!("Powtórzony gml:id „{d}”."),
            });
        }
        let mut missing_local = HashSet::new();
        let mut external = 0usize;
        for h in &hrefs {
            match h.strip_prefix('#') {
                Some(local) if !ids.contains_key(local) => {
                    missing_local.insert(local.to_string());
                }
                Some(_) => {}
                None if ids.contains_key(h) => {}
                None => external += 1,
            }
        }
        let mut missing_local: Vec<_> = missing_local.into_iter().collect();
        missing_local.sort();
        for m in missing_local {
            issues.push(Issue {
                severity: Severity::Error,
                feature: None,
                message: format!("Odwołanie „#{m}” nie wskazuje żadnego obiektu w pliku."),
            });
        }
        if external > 0 {
            issues.push(Issue {
                severity: Severity::Info,
                feature: None,
                message: format!("{external} odwołań do obiektów spoza pliku (nie sprawdzono)."),
            });
        }

        // Układy współrzędnych.
        let mut epsgs: HashSet<Option<u32>> = HashSet::new();
        for f in self.features.iter().filter(|f| f.geometry.is_some()) {
            epsgs.insert(f.srs.as_deref().and_then(epsg_from_srs));
        }
        if epsgs.contains(&None) {
            issues.push(Issue {
                severity: Severity::Warning,
                feature: None,
                message: "Część geometrii nie ma rozpoznawalnego srsName (EPSG).".into(),
            });
        }
        if epsgs.iter().flatten().count() > 1 {
            issues.push(Issue {
                severity: Severity::Warning,
                feature: None,
                message: "Geometrie są w różnych układach współrzędnych.".into(),
            });
        }

        // Geometria obiektów.
        let by_id: HashMap<&str, usize> = self
            .features
            .iter()
            .enumerate()
            .map(|(i, f)| (f.id.as_str(), i))
            .collect();
        let mut seen_parcels: HashMap<String, usize> = HashMap::new();
        for (i, f) in self.features.iter().enumerate() {
            let at = |severity, message: String| Issue {
                severity,
                feature: Some(i),
                message,
            };
            if let Some(e) = &f.geometry_error {
                issues.push(at(
                    Severity::Error,
                    format!("{} {}: błędna geometria ({e}).", f.class, f.label),
                ));
            }
            if let Some(g) = &f.geometry {
                for ring in g.rings() {
                    if ring.len() < 4 {
                        issues.push(at(
                            Severity::Error,
                            format!("{} {}: pierścień ma mniej niż 4 punkty.", f.class, f.label),
                        ));
                    } else if ring.first() != ring.last() {
                        issues.push(at(
                            Severity::Warning,
                            format!("{} {}: pierścień nie jest zamknięty.", f.class, f.label),
                        ));
                    } else if self_intersects(ring) {
                        issues.push(at(
                            Severity::Warning,
                            format!("{} {}: boki pierścienia się przecinają.", f.class, f.label),
                        ));
                    }
                }
            }

            // Reguły EGiB.
            let needs_geometry = matches!(
                f.class.as_str(),
                "EGB_DzialkaEwidencyjna" | "EGB_PunktGraniczny" | "EGB_Budynek"
            );
            if needs_geometry && f.geometry.is_none() && f.geometry_error.is_none() {
                issues.push(at(
                    Severity::Error,
                    format!("{} {}: brak geometrii.", f.class, f.label),
                ));
            }
            if f.class == "EGB_DzialkaEwidencyjna" {
                let el = root.element_at(&self.paths[i]);
                let idd = el
                    .and_then(|e| e.child("idDzialki"))
                    .map(|c| c.text().trim().to_string())
                    .unwrap_or_default();
                match idd.as_str() {
                    "" => issues.push(at(
                        Severity::Warning,
                        format!("Działka {}: brak idDzialki.", f.id),
                    )),
                    _ => {
                        if let Some(prev) = seen_parcels.insert(idd.clone(), i) {
                            issues.push(at(
                                Severity::Error,
                                format!(
                                    "Działka {idd} występuje dwukrotnie (obiekty {} i {}).",
                                    prev + 1,
                                    i + 1
                                ),
                            ));
                        }
                    }
                }
                let refs = el.into_iter().flat_map(|e| {
                    e.elements()
                        .filter(|c| c.local_name() == "punktGranicyDzialki")
                        .filter_map(|c| c.attr_local("href"))
                });
                if let Some(g) = &f.geometry {
                    let vertices = g.vertices();
                    for r in refs {
                        let target = r.trim_start_matches('#');
                        let Some(&pi) = by_id.get(target) else {
                            continue;
                        };
                        let Some(Geometry::Point(p)) = &self.features[pi].geometry else {
                            continue;
                        };
                        let on_boundary = vertices
                            .iter()
                            .any(|v| (v[0] - p[0]).hypot(v[1] - p[1]) < 0.01);
                        if !on_boundary {
                            issues.push(at(
                                Severity::Warning,
                                format!(
                                    "Działka {}: punkt graniczny {} nie leży w wierzchołku granicy (tolerancja 1 cm).",
                                    f.label, self.features[pi].label
                                ),
                            ));
                        }
                    }
                }
            }
        }
        self.issues = issues;
    }
}

fn walk(e: &Element, f: &mut impl FnMut(&Element)) {
    f(e);
    for c in e.elements() {
        walk(c, f);
    }
}

fn collect_features(
    e: &Element,
    path: &mut Vec<usize>,
    names: &GmlNames,
    gml_id: &impl Fn(&Element) -> Option<String>,
    out: &mut Vec<Vec<usize>>,
) {
    for (i, child) in e.children.iter().enumerate() {
        let xml::Node::Element(c) = child else {
            continue;
        };
        path.push(i);
        if is_member(c.local_name()) {
            for (j, m) in c.children.iter().enumerate() {
                if let xml::Node::Element(feature) = m {
                    if !names.is_geometry(feature) {
                        path.push(j);
                        out.push(path.clone());
                        path.pop();
                    }
                }
            }
        } else if gml_id(c).is_some() && !names.is_geometry(c) && !names.is_gml(c) {
            // Obiekt bezpośrednio pod kolekcją (bez elementu member).
            out.push(path.clone());
        }
        path.pop();
    }
}

/// Atrybuty z elementu: tekst, odwołanie `xlink:href` albo spłaszczone elementy zagnieżdżone.
fn properties(e: &Element, key: String, path: Vec<usize>, out: &mut Vec<Property>) {
    if let Some(href) = e.attr_local("href") {
        out.push(Property {
            key,
            value: href,
            editable: false,
            path,
        });
        return;
    }
    if !e.has_elements() {
        out.push(Property {
            key,
            value: e.text().trim().to_string(),
            editable: true,
            path,
        });
        return;
    }
    for (i, child) in e.children.iter().enumerate() {
        let xml::Node::Element(c) = child else {
            continue;
        };
        let mut p = path.clone();
        p.push(i);
        // Elementy typów (np. BT_Identyfikator, EGB_Klasouzytek) nie dokładają się do nazwy;
        // liście (np. OFU) zawsze tak, nawet pisane wielką literą.
        let name = c.local_name();
        let k = if name.starts_with(char::is_uppercase) && c.has_elements() {
            key.clone()
        } else {
            format!("{key}.{name}")
        };
        properties(c, k, p, out);
    }
}

/// Atrybuty o tej samej nazwie (np. wiele `punktGranicyDzialki`) łączy w jeden, tylko do odczytu.
fn merge_duplicates(props: &mut Vec<Property>) {
    let mut merged: Vec<Property> = Vec::with_capacity(props.len());
    for p in props.drain(..) {
        match merged.iter_mut().find(|m| m.key == p.key) {
            Some(m) => {
                m.value = format!("{}; {}", m.value, p.value);
                m.editable = false;
            }
            None => merged.push(p),
        }
    }
    *props = merged;
}

/// Identyfikatory EGiB mają postać `146518_8.0101.12/1`; etykietą jest część po ostatniej kropce.
/// Punkt graniczny pozyskany pomiarem (SPD = 1) i spełniający warunki dokładnościowe (ISD = 1).
pub const VARIANT_ACCURATE: u8 = 1;
/// Punkt graniczny stabilizowany (STB = 3, 4, 5 lub 6).
pub const VARIANT_STABILIZED: u8 = 2;
/// Obiekt GESUT o statusie „projektowany”.
pub const VARIANT_PLANNED: u8 = 4;
/// Budynek podziemny (bez kondygnacji nadziemnych).
pub const VARIANT_UNDERGROUND: u8 = 8;
/// Przewód GESUT nadziemny (napowietrzny).
pub const VARIANT_OVERHEAD: u8 = 16;

fn child_text(el: &Element, name: &str) -> String {
    el.child(name)
        .filter(|c| !c.has_elements())
        .map(|c| c.text().trim().to_string())
        .unwrap_or_default()
}

struct Described {
    label: String,
    variant: u8,
    code: String,
}

/// Etykieta, wariant stylu i kod znaku umownego obiektu według katalogu znaków mapy zasadniczej
/// (Dz.U. 2021 poz. 1385, zał. 4). Kody powstają z klasy i atrybutu rodzaju, tak jak w katalogu:
/// BDOT500 `OT` + litera grupy + rodzaj, GESUT `SU` + sieć + rodzaj urządzenia, EGiB `EG..`.
fn describe(class: &str, el: &Element) -> Described {
    let t = |n: &str| child_text(el, n);
    let id_label = || {
        LABEL_KEYS
            .iter()
            .find_map(|k| {
                let v = t(k);
                (!v.is_empty()).then(|| short_label(k, &v))
            })
            .unwrap_or_default()
    };
    let mut variant = 0;
    let label = match class {
        "EGB_PunktGraniczny" => {
            if t("sposobPozyskania") == "1" && t("spelnienieWarunkowDokl") == "1" {
                variant |= VARIANT_ACCURATE;
            }
            if matches!(t("rodzajStabilizacji").as_str(), "3" | "4" | "5" | "6") {
                variant |= VARIANT_STABILIZED;
            }
            id_label()
        }
        // Opis budynku: rodzaj wg KŚT i numer najwyższej kondygnacji nadziemnej, np. „m3”.
        "EGB_Budynek" | "OT_BudynekNiewykazanyWEGIB" => {
            let above = t("liczbaKondygnacjiNadziemnych");
            if above.parse::<i64>().is_ok_and(|n| n == 0) {
                variant |= VARIANT_UNDERGROUND;
            }
            let kst = if class == "EGB_Budynek" {
                t("rodzajWgKST")
            } else {
                t("rodzajKST")
            };
            format!("{kst}{above}")
        }
        // Pikiety BDOT500 opisane wysokością.
        "OT_Rzedna" => t("rzedna"),
        "EGB_KonturUzytkuGruntowego" => t("OFU"),
        "EGB_KonturKlasyfikacyjny" => format!("{}{}", t("OZU"), t("OZK")),
        c if c.starts_with("GES_") => {
            let planned = t("status") == "p";
            if planned {
                variant |= VARIANT_PLANNED;
            }
            if t("przebieg") == "n" {
                variant |= VARIANT_OVERHEAD;
            }
            match gesut_network(c) {
                Some(net) if c.starts_with("GES_Przewod") => {
                    conduit_label(net, &t, planned, variant)
                }
                Some(net) if planned => format!("proj. {}", net.to_ascii_lowercase()),
                _ => id_label(),
            }
        }
        _ => id_label(),
    };
    Described {
        label,
        variant,
        code: symbol_code(class, &t),
    }
}

/// Litera sieci uzbrojenia w kodach znaków: W, K, E, G, C, T, S, N.
fn gesut_network(class: &str) -> Option<char> {
    const NETS: [(&str, char); 8] = [
        ("Wodociag", 'W'),
        ("Kanalizac", 'K'),
        ("Elektroenerget", 'E'),
        ("Gaz", 'G'),
        ("Cieplow", 'C'),
        ("Telekomunik", 'T'),
        ("Specjaln", 'S'),
        ("Niezidentyfikowan", 'N'),
    ];
    NETS.iter()
        .find(|(k, _)| class.contains(k))
        .map(|&(_, c)| c)
}

/// Opis przewodu (zał. 4, rozdz. 1 ust. 5–6): rodzaj sieci, typ i średnica, np. „w150”, „kd200”;
/// przewody projektowane z dopiskiem „proj.”, nadziemne elektroenergetyczne i telekomunikacyjne bez opisu.
fn conduit_label(net: char, t: &dyn Fn(&str) -> String, planned: bool, variant: u8) -> String {
    if variant & VARIANT_OVERHEAD != 0 && matches!(net, 'E' | 'T') {
        return String::new();
    }
    let mut s = String::from(net.to_ascii_lowercase());
    let typ = t("typPrzewodu");
    if !(net == 'T' && typ == "i") {
        s.push_str(&typ);
    }
    s.push_str(&t("srednica"));
    if t("eksploatacja") == "n" {
        s.push_str("-niecz.");
    }
    if planned {
        s = format!("proj. {s}");
    }
    s
}

/// Kod znaku umownego z katalogu mapy zasadniczej. Jeśli plik podaje `kodObiektu`, ma on pierwszeństwo.
fn symbol_code(class: &str, t: &dyn Fn(&str) -> String) -> String {
    let given = t("kodObiektu");
    if given.len() == 4 && given.bytes().all(|b| b.is_ascii_uppercase()) {
        return given;
    }
    let sub = |prefix: &str, attr: &str| match t(attr).as_bytes() {
        [b] if b.is_ascii_alphabetic() => format!("{prefix}{}", b.to_ascii_uppercase() as char),
        _ => String::new(),
    };
    match class {
        "EGB_JednostkaEwidencyjna" => "EGJE".into(),
        "EGB_ObrebEwidencyjny" => "EGOE".into(),
        "EGB_DzialkaEwidencyjna" => "EGDE".into(),
        "EGB_PunktGraniczny" => "EGPG".into(),
        "EGB_KonturUzytkuGruntowego" => "EGUG".into(),
        "EGB_KonturKlasyfikacyjny" => "EGKK".into(),
        "EGB_Budynek" => "EGBU".into(),
        "OT_BudynekNiewykazanyWEGIB" => if t("status") == "b" { "OTBB" } else { "OTBU" }.into(),
        "EGB_BlokBudynku" => sub("EGB", "rodzajBloku"),
        "EGB_ObiektTrwaleZwiazanyZBudynkiem" => sub("EGB", "rodzajObiektuZwiazanegoZBudynkiem"),
        "OT_BlokBudynku" => sub("OTB", "rodzajBloku"),
        "OT_ObiektTrwaleZwiazanyZBudynkami" => sub("OTB", "rodzajObiektu"),
        "OT_Budowle" => sub("OTD", "rodzajBudowli"),
        "OT_Ogrodzenia" => sub("OTO", "rodzajOgrodzenia"),
        "OT_Komunikacja" => sub("OTK", "rodzajObiektu"),
        "OT_SportIRekreacja" => sub("OTS", "rodzajObiektu"),
        "OT_ZagospodarowanieTerenu" => sub("OTZ", "rodzajObiektu"),
        "OT_Wody" => sub("OTW", "rodzajObiektu"),
        "OT_Skarpa" => sub("OTR", "rodzajSkarpy"),
        "OT_Rzedna" => sub("OTR", "rodzajRzednej"),
        "GES_UrzadzeniaTechniczneSieciSpecjalnej" => "SUSU".into(),
        "GES_UrzadzenieNiezidentyfikowane" => "SUNU".into(),
        "GES_InneUrzadzeniaTowarzyszace" => sub("SUP", "rodzajUrzadzenia"),
        "GES_UrzadzeniaTowarzyszczaceLiniowe" => sub("SUL", "rodzajUrzadzenia"),
        c if c.starts_with("GES_Przewod") => gesut_network(c)
            .map(|n| format!("SU{n}P"))
            .unwrap_or_default(),
        c if c.starts_with("GES_UrzadzeniaSiec") => match gesut_network(c) {
            Some(n) => {
                let code = sub(&format!("SU{n}"), "rodzajUrzadzenia");
                if code.is_empty() {
                    format!("SU{n}U")
                } else {
                    code
                }
            }
            None => String::new(),
        },
        _ => String::new(),
    }
}

fn short_label(key: &str, value: &str) -> String {
    if key.starts_with("id") {
        value.rsplit('.').next().unwrap_or(value).to_string()
    } else {
        value.to_string()
    }
}

/// Czy pierścień (zamknięty) ma przecinające się boki.
pub fn self_intersects(ring: &[P]) -> bool {
    let n = ring.len().saturating_sub(1);
    if n < 4 {
        return false;
    }
    let cross = |o: P, a: P, b: P| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let hit = |p1: P, p2: P, q1: P, q2: P| {
        let (d1, d2) = (cross(q1, q2, p1), cross(q1, q2, p2));
        let (d3, d4) = (cross(p1, p2, q1), cross(p1, p2, q2));
        d1 * d2 < 0.0 && d3 * d4 < 0.0
    };
    (0..n).any(|i| {
        (i + 2..n)
            .filter(|&j| !(i == 0 && j == n - 1))
            .any(|j| hit(ring[i], ring[i + 1], ring[j], ring[j + 1]))
    })
}

/// Punkt projektu do zapisu w GML.
pub struct PointOut<'a> {
    pub id: &'a str,
    pub x: f64,
    pub y: f64,
    pub h: Option<f64>,
    pub code: &'a str,
}

/// `gml:id` musi być nazwą XML: litery, cyfry, `_ - .`, bez cyfry na początku.
fn ncname(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Zapisuje punkty jako prostą kolekcję GML 3.2 (schemat `urn:pikieta:punkty:1.0`).
pub fn points_to_gml(points: &[PointOut], epsg: u32, decimals: usize) -> String {
    let f = |v: f64| round_half_even(v, decimals);
    let srs = format!("urn:ogc:def:crs:EPSG::{epsg}");
    let mut used = HashSet::new();
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <gml:FeatureCollection xmlns:gml=\"http://www.opengis.net/gml/3.2\" \
         xmlns:pkt=\"urn:pikieta:punkty:1.0\" gml:id=\"pikieta.punkty\">\n",
    );
    for p in points {
        let base = format!("p.{}", ncname(p.id));
        let mut id = base.clone();
        let mut n = 2;
        while !used.insert(id.clone()) {
            id = format!("{base}.{n}");
            n += 1;
        }
        out.push_str(&format!(
            "  <gml:featureMember>\n    <pkt:Punkt gml:id=\"{id}\">\n      <pkt:numer>{}</pkt:numer>\n",
            xml::escape(p.id)
        ));
        if !p.code.is_empty() {
            out.push_str(&format!(
                "      <pkt:kod>{}</pkt:kod>\n",
                xml::escape(p.code)
            ));
        }
        if let Some(h) = p.h {
            out.push_str(&format!("      <pkt:h>{}</pkt:h>\n", f(h)));
        }
        out.push_str(&format!(
            "      <pkt:geometria><gml:Point gml:id=\"{id}.g\" srsName=\"{srs}\" srsDimension=\"2\">\
             <gml:pos>{} {}</gml:pos></gml:Point></pkt:geometria>\n    </pkt:Punkt>\n  </gml:featureMember>\n",
            f(p.x),
            f(p.y)
        ));
    }
    out.push_str("</gml:FeatureCollection>\n");
    out
}

impl GmlNames {
    /// Atrybut w przestrzeni nazw GML (np. `gml:id`).
    pub fn is_gml_attr(&self, name: &str) -> bool {
        name.split_once(':')
            .is_some_and(|(p, _)| self.has_prefix(p))
    }
}

#[cfg(test)]
mod tests;
