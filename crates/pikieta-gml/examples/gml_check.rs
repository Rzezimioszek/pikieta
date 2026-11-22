//! Diagnostyka plików GML: czas odczytu, zapis bajt w bajt, klasy, błędy geometrii,
//! uwagi walidacji i pokrycie kodami znaków umownych.
//!
//! `cargo run --release -p pikieta-gml --example gml_check -- plik.gml [...]`

use pikieta_gml::{GmlDocument, Severity};
use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    for path in std::env::args().skip(1) {
        println!("=== {path}");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("  błąd odczytu: {e}");
                continue;
            }
        };
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => {
                println!("  plik nie jest w UTF-8: {e}");
                continue;
            }
        };
        let t0 = Instant::now();
        let doc = match GmlDocument::parse(&text) {
            Ok(d) => d,
            Err(e) => {
                println!("  błąd parsowania: {e:?}");
                continue;
            }
        };
        let parse = t0.elapsed();
        let t1 = Instant::now();
        let out = doc.to_xml();
        let write = t1.elapsed();
        let same = out == text;
        println!(
            "  {:.1} MB, odczyt {:.2} s, zapis {:.2} s, bajt w bajt: {}",
            text.len() as f64 / 1e6,
            parse.as_secs_f64(),
            write.as_secs_f64(),
            if same { "tak" } else { "NIE" }
        );
        if !same {
            let pos = out
                .bytes()
                .zip(text.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(out.len().min(text.len()));
            let ctx = |s: &str| {
                s.get(pos.saturating_sub(60)..(pos + 60).min(s.len()))
                    .unwrap_or("")
                    .replace('\n', "⏎")
            };
            println!(
                "  pierwsza różnica na bajcie {pos}:\n    oryginał: {}\n    zapis:    {}",
                ctx(&text),
                ctx(&out)
            );
        }
        let t2 = Instant::now();
        let buf = doc.render_buffer(Some(2180));
        println!(
            "  epsg {:?}, bufor geometrii {:.1} MB w {:.2} s",
            doc.epsg(),
            buf.len() as f64 / 1e6,
            t2.elapsed().as_secs_f64()
        );
        let t3 = Instant::now();
        let texts = doc.carto_texts();
        let json = doc.texts_json(Some(2180));
        let sample: Vec<&str> = texts.iter().take(6).map(|t| t.text.as_str()).collect();
        println!(
            "  teksty kartograficzne: {} (z obiektem opisywanym {}), JSON {:.1} MB w {:.2} s, np. {sample:?}",
            texts.len(),
            texts.iter().filter(|t| t.target.is_some()).count(),
            json.len() as f64 / 1e6,
            t3.elapsed().as_secs_f64()
        );

        // Klasy: liczba obiektów, bez geometrii, błędy geometrii, kody.
        #[derive(Default)]
        struct C {
            n: usize,
            no_geom: usize,
            err: usize,
            codes: BTreeMap<String, usize>,
            labels: Vec<String>,
        }
        let mut classes: BTreeMap<&str, C> = BTreeMap::new();
        let mut errors: BTreeMap<String, usize> = BTreeMap::new();
        for f in &doc.features {
            let c = classes.entry(f.class.as_str()).or_default();
            c.n += 1;
            if f.geometry.is_none() && f.geometry_error.is_none() {
                c.no_geom += 1;
            }
            if let Some(e) = &f.geometry_error {
                c.err += 1;
                *errors.entry(e.chars().take(90).collect()).or_default() += 1;
            }
            *c.codes
                .entry(if f.code.is_empty() {
                    "—".into()
                } else {
                    f.code.clone()
                })
                .or_default() += 1;
            if c.labels.len() < 3 && f.label != f.id {
                c.labels.push(f.label.clone());
            }
        }
        println!("  obiekty: {}", doc.features.len());
        for (name, c) in &classes {
            let codes: Vec<String> = c.codes.iter().map(|(k, v)| format!("{k}:{v}")).collect();
            println!(
                "    {name:42} {:>7}  bez geom. {:>6}  błędy {:>5}  kody [{}]  etykiety {:?}",
                c.n,
                c.no_geom,
                c.err,
                codes.join(" "),
                c.labels
            );
        }
        if !errors.is_empty() {
            println!("  błędy geometrii:");
            for (e, n) in &errors {
                println!("    {n:>6}× {e}");
            }
        }
        let mut issues: BTreeMap<(String, String), usize> = BTreeMap::new();
        for i in &doc.issues {
            let sev = match i.severity {
                Severity::Error => "błąd",
                Severity::Warning => "ostrzeżenie",
                Severity::Info => "info",
            };
            // Grupowanie po treści bez numerów i identyfikatorów.
            let key: String = i
                .message
                .split([' ', ':'])
                .take(6)
                .collect::<Vec<_>>()
                .join(" ");
            *issues.entry((sev.to_string(), key)).or_default() += 1;
        }
        println!("  uwagi walidacji: {}", doc.issues.len());
        for ((sev, msg), n) in issues.iter().take(25) {
            println!("    {n:>6}× [{sev}] {msg}…");
        }
    }
}
