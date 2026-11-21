//! Raport obliczeń: model danych i składanie do tekstu oraz HTML.
//!
//! Wartości przychodzą już sformatowane (zaokrąglenia robi `pikieta-core`),
//! więc raport odpowiada tylko za układ: nagłówek, parametry, tabele i wyniki.

use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// Cały raport.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Report {
    pub title: String,
    /// Data sporządzenia, już sformatowana.
    pub date: String,
    /// Nazwa układu współrzędnych.
    pub crs: String,
    /// Uwagi pod nagłówkiem, np. jednostki.
    pub notes: Vec<String>,
    pub entries: Vec<Entry>,
}

/// Jedna pozycja raportu, czyli jedno zapisane obliczenie.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub title: String,
    /// Dane wejściowe: pary nazwa, wartość.
    pub params: Vec<(String, String)>,
    pub table: Option<Table>,
    /// Wyniki: pary nazwa, wartość.
    pub results: Vec<(String, String)>,
    pub notes: Vec<String>,
    /// Wpisy z wcześniejszych wersji, zapisane jako gotowe wiersze tekstu.
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Table {
    pub head: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Kolumny wyrównane do prawej (liczby). Brak wpisu oznacza lewą.
    pub right: Vec<bool>,
}

impl Table {
    fn is_right(&self, col: usize) -> bool {
        self.right.get(col).copied().unwrap_or(false)
    }

    fn columns(&self) -> usize {
        self.head
            .len()
            .max(self.rows.iter().map(Vec::len).max().unwrap_or(0))
    }
}

/// Odczytuje raport z JSON.
pub fn from_json(json: &str) -> Result<Report, String> {
    serde_json::from_str(json).map_err(|e| format!("niepoprawne dane raportu: {e}"))
}

fn width(s: &str) -> usize {
    s.chars().count()
}

fn pad(s: &str, w: usize, right: bool) -> String {
    let fill = " ".repeat(w.saturating_sub(width(s)));
    if right {
        format!("{fill}{s}")
    } else {
        format!("{s}{fill}")
    }
}

fn cell(row: &[String], c: usize) -> &str {
    row.get(c).map(String::as_str).unwrap_or("")
}

fn table_text(t: &Table, indent: &str, out: &mut String) {
    let cols = t.columns();
    let widths: Vec<usize> = (0..cols)
        .map(|c| {
            t.rows
                .iter()
                .map(|r| width(cell(r, c)))
                .chain([width(cell(&t.head, c))])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |row: &[String]| {
        let cells: Vec<String> = (0..cols)
            .map(|c| pad(cell(row, c), widths[c], t.is_right(c)))
            .collect();
        format!("{indent}{}", cells.join("  ").trim_end())
    };
    let _ = writeln!(out, "{}", line(&t.head));
    let rule = widths.iter().sum::<usize>() + 2 * cols.saturating_sub(1);
    let _ = writeln!(out, "{indent}{}", "-".repeat(rule));
    for row in &t.rows {
        let _ = writeln!(out, "{}", line(row));
    }
}

fn pairs_text(pairs: &[(String, String)], indent: &str, out: &mut String) {
    let w = pairs.iter().map(|(k, _)| width(k)).max().unwrap_or(0);
    for (k, v) in pairs {
        let _ = writeln!(out, "{indent}{}  {v}", pad(&format!("{k}:"), w + 1, false));
    }
}

fn header(r: &Report) -> Vec<(String, String)> {
    let mut head = vec![("Data".to_string(), r.date.clone())];
    if !r.crs.is_empty() {
        head.push(("Układ współrzędnych".to_string(), r.crs.clone()));
    }
    head
}

/// Raport jako zwykły tekst o stałej szerokości znaków.
pub fn to_text(r: &Report) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}", r.title);
    let _ = writeln!(out, "{}", "=".repeat(width(&r.title)));
    pairs_text(&header(r), "", &mut out);
    for n in &r.notes {
        let _ = writeln!(out, "{n}");
    }
    for (i, e) in r.entries.iter().enumerate() {
        let _ = writeln!(out, "\n{}. {}", i + 1, e.title);
        let ind = "   ";
        for l in &e.lines {
            let _ = writeln!(out, "{ind}{l}");
        }
        pairs_text(&e.params, ind, &mut out);
        if let Some(t) = &e.table {
            if !e.params.is_empty() {
                out.push('\n');
            }
            table_text(t, ind, &mut out);
        }
        if !e.results.is_empty() {
            if !e.params.is_empty() || e.table.is_some() {
                out.push('\n');
            }
            pairs_text(&e.results, ind, &mut out);
        }
        for n in &e.notes {
            let _ = writeln!(out, "{ind}{n}");
        }
    }
    out
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn pairs_html(pairs: &[(String, String)], class: &str, out: &mut String) {
    if pairs.is_empty() {
        return;
    }
    let _ = write!(out, "<dl class=\"{class}\">");
    for (k, v) in pairs {
        let _ = write!(out, "<dt>{}</dt><dd>{}</dd>", esc(k), esc(v));
    }
    out.push_str("</dl>");
}

const STYLE: &str = "\
@page{size:A4;margin:16mm 14mm}\
*{box-sizing:border-box}\
html{color-scheme:light}\
body{margin:0 auto;max-width:190mm;padding:24px;font:10.5pt/1.45 system-ui,'Segoe UI',sans-serif;color:#111;background:#fff}\
@media print{body{padding:0}}\
header{border-bottom:2px solid #111;padding-bottom:8px;margin-bottom:18px}\
h1{font-size:17pt;margin:0 0 6px}\
h2{font-size:11.5pt;margin:0 0 8px}\
section{margin:0 0 18px;break-inside:avoid}\
dl{display:grid;grid-template-columns:max-content 1fr;gap:2px 14px;margin:0 0 8px}\
dt{color:#555}dd{margin:0;font-variant-numeric:tabular-nums}\
dl.results dd{font-weight:650}\
table{border-collapse:collapse;margin:4px 0 10px;font-variant-numeric:tabular-nums}\
th,td{border:1px solid #bbb;padding:3px 8px;text-align:left}\
th{background:#eee;font-weight:600}\
td.r,th.r{text-align:right}\
pre{margin:0 0 8px;font:9.5pt/1.45 ui-monospace,Consolas,monospace;white-space:pre-wrap}\
p.note{margin:2px 0;color:#555}\
footer{margin-top:24px;border-top:1px solid #bbb;padding-top:6px;font-size:8.5pt;color:#666}";

/// Raport jako samodzielny dokument HTML, przygotowany do druku na A4.
pub fn to_html(r: &Report, generator: &str) -> String {
    let mut out = String::new();
    let title = esc(&r.title);
    let _ = write!(
        out,
        "<!doctype html><html lang=\"pl\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{title}</title><style>{STYLE}</style></head><body><header><h1>{title}</h1>"
    );
    pairs_html(&header(r), "head", &mut out);
    for n in &r.notes {
        let _ = write!(out, "<p class=\"note\">{}</p>", esc(n));
    }
    out.push_str("</header>");

    for (i, e) in r.entries.iter().enumerate() {
        let _ = write!(out, "<section><h2>{}. {}</h2>", i + 1, esc(&e.title));
        if !e.lines.is_empty() {
            let _ = write!(out, "<pre>{}</pre>", esc(&e.lines.join("\n")));
        }
        pairs_html(&e.params, "params", &mut out);
        if let Some(t) = &e.table {
            let cls = |c: usize| if t.is_right(c) { " class=\"r\"" } else { "" };
            out.push_str("<table><thead><tr>");
            for (c, h) in t.head.iter().enumerate() {
                let _ = write!(out, "<th{}>{}</th>", cls(c), esc(h));
            }
            out.push_str("</tr></thead><tbody>");
            for row in &t.rows {
                out.push_str("<tr>");
                for (c, v) in row.iter().enumerate() {
                    let _ = write!(out, "<td{}>{}</td>", cls(c), esc(v));
                }
                out.push_str("</tr>");
            }
            out.push_str("</tbody></table>");
        }
        pairs_html(&e.results, "results", &mut out);
        for n in &e.notes {
            let _ = write!(out, "<p class=\"note\">{}</p>", esc(n));
        }
        out.push_str("</section>");
    }
    let _ = write!(out, "<footer>{}</footer></body></html>", esc(generator));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Report {
        Report {
            title: "Raport obliczeń".into(),
            date: "21.11.2026".into(),
            crs: "PL-2000 strefa 7".into(),
            notes: vec!["Współrzędne w metrach.".into()],
            entries: vec![
                Entry {
                    title: "Pole <1-2-3>".into(),
                    params: vec![("Wierzchołki".into(), "3".into())],
                    table: Some(Table {
                        head: vec!["Nr".into(), "X".into()],
                        rows: vec![
                            vec!["1".into(), "5788400.00".into()],
                            vec!["22".into(), "1.50".into()],
                        ],
                        right: vec![false, true],
                    }),
                    results: vec![("Pole".into(), "10000.00 m²".into())],
                    ..Default::default()
                },
                Entry {
                    title: "Stary wpis".into(),
                    lines: vec!["Azymut: 50.0000 g".into()],
                    ..Default::default()
                },
            ],
        }
    }

    #[test]
    fn text_layout() {
        let expected = "\
Raport obliczeń
===============
Data:                 21.11.2026
Układ współrzędnych:  PL-2000 strefa 7
Współrzędne w metrach.

1. Pole <1-2-3>
   Wierzchołki:  3

   Nr           X
   --------------
   1   5788400.00
   22        1.50

   Pole:  10000.00 m²

2. Stary wpis
   Azymut: 50.0000 g
";
        assert_eq!(to_text(&sample()), expected);
    }

    #[test]
    fn html_is_escaped_and_complete() {
        let h = to_html(&sample(), "Pikieta 0.1.0");
        assert!(h.starts_with("<!doctype html>"));
        assert!(h.contains("<h2>1. Pole &lt;1-2-3&gt;</h2>"));
        assert!(h.contains("<td class=\"r\">5788400.00</td>"));
        assert!(h.contains("<pre>Azymut: 50.0000 g</pre>"));
        assert!(h.contains("<dd>10000.00 m²</dd>"));
        assert!(h.ends_with("<footer>Pikieta 0.1.0</footer></body></html>"));
        assert!(!h.contains("<1-2-3>"));
    }

    #[test]
    fn json_round_trip_and_old_entries() {
        let r = sample();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(from_json(&json).unwrap(), r);
        // Wpis w starym formacie: tylko tytuł i wiersze.
        let old = r#"{"title":"R","entries":[{"title":"A","lines":["x"]}]}"#;
        assert_eq!(from_json(old).unwrap().entries[0].lines, vec!["x"]);
        assert!(from_json("{").is_err());
    }
}
