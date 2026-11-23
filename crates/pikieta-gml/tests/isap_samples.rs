//! Przykładowe pliki GML z folderu `isap` (planowanie przestrzenne APP 1.0 i 2.0):
//! odczyt bez błędów geometrii, zapis bajt w bajt i kody znaków wydzieleń.

use pikieta_gml::GmlDocument;
use std::path::Path;

#[test]
fn isap_sample_files_round_trip() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../isap");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut checked = 0;
    for e in entries.flatten() {
        let path = e.path();
        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "gml" {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let doc = GmlDocument::parse(&text).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert_eq!(
            doc.to_xml(),
            text,
            "{}: zapis różni się od odczytu",
            path.display()
        );
        for f in &doc.features {
            assert!(
                f.geometry_error.is_none(),
                "{}: {} {:?}",
                path.display(),
                f.id,
                f.geometry_error
            );
        }
        // Każdy plik APP ma akt planowania z zasięgiem.
        assert!(
            doc.features
                .iter()
                .any(|f| f.code == "APP-AKT" && f.geometry.is_some()),
            "{}: brak zasięgu aktu planowania",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked == 0 || checked >= 10,
        "sprawdzono tylko {checked} plików"
    );
}
