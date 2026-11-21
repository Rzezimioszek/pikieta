use super::*;

const EGIB: &str = include_str!("../tests/data/egib-przyklad.gml");

fn doc() -> GmlDocument {
    GmlDocument::parse(EGIB).unwrap()
}

fn find<'a>(d: &'a GmlDocument, id: &str) -> &'a Feature {
    d.features.iter().find(|f| f.id == id).unwrap()
}

fn prop(d: &GmlDocument, id: &str, key: &str) -> Option<String> {
    let i = d.features.iter().position(|f| f.id == id).unwrap();
    d.property(i, key)
}

#[test]
fn unchanged_export_is_identical() {
    assert_eq!(doc().to_xml(), EGIB);
}

#[test]
fn finds_egib_features() {
    let d = doc();
    let classes: Vec<&str> = d.features.iter().map(|f| f.class.as_str()).collect();
    assert_eq!(classes.len(), 12);
    let count = |c: &str| classes.iter().filter(|x| **x == c).count();
    assert_eq!(count("EGB_DzialkaEwidencyjna"), 2);
    assert_eq!(count("EGB_PunktGraniczny"), 6);
    assert_eq!(count("EGB_Budynek"), 1);
    assert_eq!(count("EGB_ObrebEwidencyjny"), 1);
    assert_eq!(d.srs().as_deref(), Some("urn:ogc:def:crs:EPSG::2178"));
    assert_eq!(
        d.issues,
        vec![Issue {
            severity: Severity::Info,
            feature: None,
            message: "1 odwołań do obiektów spoza pliku (nie sprawdzono).".into(),
        }]
    );
}

#[test]
fn parcel_attributes_and_label() {
    let d = doc();
    let f = find(&d, "TEST.Dz.12_1");
    assert_eq!(f.label, "12/1");
    let p = |k: &str| prop(&d, "TEST.Dz.12_1", k);
    assert_eq!(p("idDzialki").as_deref(), Some("999999_9.0101.12/1"));
    assert_eq!(p("idIIP.lokalnyId").as_deref(), Some("dz-12-1"));
    assert_eq!(p("klasouzytekWGranicachDzialki.OFU").as_deref(), Some("B"));
    assert_eq!(p("lokalizacjaDzialki").as_deref(), Some("#TEST.Obreb.0101"));
    assert_eq!(
        p("punktGranicyDzialki").as_deref(),
        Some("#TEST.PG.1; #TEST.PG.2; #TEST.PG.3; #TEST.PG.4")
    );
    assert_eq!(p("uwagi").as_deref(), Some("Opis z <nawiasami> & znakami"));
    assert_eq!(
        prop(&d, "TEST.Obreb.0101", "nazwaWlasna").as_deref(),
        Some("Obręb Testowy & Przykładowy")
    );
    assert_eq!(prop(&d, "TEST.Dz.12_2", "numerKW").as_deref(), Some(""));
    assert_eq!(find(&d, "TEST.PG.3").label, "103");
}

#[test]
fn geometries() {
    let d = doc();
    let parcel = find(&d, "TEST.Dz.12_1").geometry.as_ref().unwrap();
    assert_eq!(parcel.rings()[0][1], [5788500.0, 7500800.0]);
    let building = find(&d, "TEST.Bud.1").geometry.as_ref().unwrap();
    assert_eq!(building.rings().len(), 2);
    let contour = find(&d, "TEST.KU.1").geometry.as_ref().unwrap();
    let ring = &contour.rings()[0];
    assert!(ring.len() > 20, "łuk powinien być zagęszczony");
    assert_eq!(ring.first(), ring.last());
    let class3d = find(&d, "TEST.KK.1").geometry.as_ref().unwrap();
    assert_eq!(class3d.rings()[0][2], [5788500.0, 7501000.0]);
    assert_eq!(
        find(&d, "TEST.PG.5").geometry,
        Some(Geometry::Point([5788500.0, 7501000.0]))
    );
}

#[test]
fn editing_changes_only_that_element() {
    let mut d = doc();
    let i = d
        .features
        .iter()
        .position(|f| f.id == "TEST.Dz.12_2")
        .unwrap();
    d.set_property(i, "numerKW", "TE1S/00099999/1").unwrap();
    d.set_property(i, "idDzialki", "999999_9.0101.12/3")
        .unwrap();
    assert_eq!(d.features[i].label, "12/3");
    assert!(d.set_property(i, "punktGranicyDzialki", "x").is_err());
    assert!(d.set_property(i, "brak", "x").is_err());

    let xml = d.to_xml();
    assert!(xml.contains("<egb:numerKW>TE1S/00099999/1</egb:numerKW>"));
    let changed: Vec<_> = EGIB
        .lines()
        .zip(xml.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(changed.len(), 2, "{changed:#?}");

    let again = GmlDocument::parse(&xml).unwrap();
    assert_eq!(
        prop(&again, "TEST.Dz.12_2", "idDzialki").as_deref(),
        Some("999999_9.0101.12/3")
    );
}

#[test]
fn validation_finds_problems() {
    let bad = EGIB
        .replace("#TEST.PG.5\"", "#TEST.PG.99\"")
        .replace("<gml:pos>5788500.00 7500800.00</gml:pos>", "<gml:pos>5788500.50 7500800.00</gml:pos>")
        .replace("gml:id=\"TEST.PG.6\"", "gml:id=\"TEST.PG.5\"")
        .replace(
            "5788400.00 7500800.00 5788500.00 7500800.00 5788500.00 7500900.00 5788400.00 7500900.00 5788400.00 7500800.00",
            "5788400.00 7500800.00 5788500.00 7500900.00 5788500.00 7500800.00 5788400.00 7500900.00 5788400.00 7500800.00",
        );
    let d = GmlDocument::parse(&bad).unwrap();
    let msgs: Vec<&str> = d.issues.iter().map(|i| i.message.as_str()).collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("Powtórzony gml:id „TEST.PG.5”")),
        "{msgs:#?}"
    );
    assert!(
        msgs.iter().any(|m| m.contains("„#TEST.PG.99”")),
        "{msgs:#?}"
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("12/1: boki pierścienia się przecinają")),
        "{msgs:#?}"
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("punkt graniczny 102 nie leży")),
        "{msgs:#?}"
    );
}

#[test]
fn broken_xml_and_geometry() {
    assert!(GmlDocument::parse("<a><b></a>").is_err());
    let d = GmlDocument::parse(&EGIB.replace(
        "<gml:pos>5788400.00 7501000.00</gml:pos>",
        "<gml:pos>5788400.00 x</gml:pos>",
    ))
    .unwrap();
    let f = find(&d, "TEST.PG.6");
    assert!(f.geometry.is_none());
    assert!(f.geometry_error.as_deref().unwrap().contains("„x”"));
    assert!(d
        .issues
        .iter()
        .any(|i| i.severity == Severity::Error && i.message.contains("106")));
}

#[test]
fn srs_names() {
    assert_eq!(epsg_from_srs("urn:ogc:def:crs:EPSG::2178"), Some(2178));
    assert_eq!(epsg_from_srs("EPSG:2180"), Some(2180));
    assert_eq!(
        epsg_from_srs("http://www.opengis.net/def/crs/EPSG/0/2177"),
        Some(2177)
    );
    assert_eq!(epsg_from_srs("urn:ogc:def:crs:EPSG:6.9:25834"), Some(25834));
    assert_eq!(epsg_from_srs("CRS:84"), None);
}

#[test]
fn points_writer_round_trip() {
    let pts = [
        PointOut {
            id: "1",
            x: 5788400.125,
            y: 7500800.135,
            h: Some(110.5),
            code: "granica & płot",
        },
        PointOut {
            id: "1",
            x: 1.0,
            y: 2.0,
            h: None,
            code: "",
        },
        PointOut {
            id: "S 1/2",
            x: 3.0,
            y: 4.0,
            h: None,
            code: "",
        },
    ];
    let xml = points_to_gml(&pts, 2178, 2);
    let d = GmlDocument::parse(&xml).unwrap();
    assert_eq!(d.issues, vec![]);
    assert_eq!(d.features.len(), 3);
    assert_eq!(
        d.features[0].geometry,
        Some(Geometry::Point([5788400.12, 7500800.14]))
    );
    assert_eq!(d.property(0, "kod").as_deref(), Some("granica & płot"));
    assert_eq!(d.property(0, "h").as_deref(), Some("110.50"));
    assert_eq!(d.features[0].label, "1");
    assert_eq!(d.features[1].id, "p.1.2");
    assert_eq!(d.features[2].id, "p.S_1_2");
    assert_eq!(
        epsg_from_srs(d.features[2].srs.as_deref().unwrap()),
        Some(2178)
    );
}
