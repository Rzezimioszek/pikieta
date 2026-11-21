//! Pomiar czasu wczytywania dużego pliku GML: `cargo run --release -p pikieta-gml --example pomiar -- plik.gml`.

use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("podaj ścieżkę do pliku GML");
    let t = Instant::now();
    let text = std::fs::read_to_string(&path).expect("odczyt pliku");
    println!("odczyt: {:?} ({} MB)", t.elapsed(), text.len() / 1_000_000);

    let t = Instant::now();
    let tree = pikieta_gml::xml::parse(&text).expect("XML");
    println!("samo drzewo XML: {:?}", t.elapsed());
    drop(tree);

    let t = Instant::now();
    let doc = pikieta_gml::GmlDocument::parse(&text).expect("parsowanie");
    println!("parsowanie + obiekty + walidacja: {:?}", t.elapsed());
    let vertices: usize = doc
        .features
        .iter()
        .filter_map(|f| f.geometry.as_ref())
        .map(|g| g.vertices().len())
        .sum();
    println!(
        "obiekty: {}, wierzchołki: {vertices}, uwagi: {}",
        doc.features.len(),
        doc.issues.len()
    );

    let t = Instant::now();
    let buf = doc.render_buffer(Some(2180));
    println!(
        "bufor mapy (z przeliczeniem do 2180): {:?}, {} MB",
        t.elapsed(),
        buf.len() / 1_000_000
    );
    let t = Instant::now();
    let idx = doc.index_json();
    println!("indeks: {:?}, {} MB", t.elapsed(), idx.len() / 1_000_000);
    let t = Instant::now();
    let table = doc.class_table_json("EGB_DzialkaEwidencyjna");
    println!(
        "tabela działek: {:?}, {} MB",
        t.elapsed(),
        table.len() / 1_000_000
    );

    let t = Instant::now();
    let xml = doc.to_xml();
    println!("zapis: {:?}, identyczny: {}", t.elapsed(), xml == text);
}
