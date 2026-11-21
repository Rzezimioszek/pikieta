//! Import i eksport tekstowych wykazów współrzędnych.
//!
//! Format wiersza: `numer  X  Y  [H]  [kod]` albo `numer  Y  X  [H]  [kod]`.
//! Pola mogą być rozdzielone spacjami, tabulatorami, średnikami albo (gdy nie ma
//! innych separatorów) przecinkami. Liczby mogą mieć kropkę albo przecinek dziesiętny.

/// Punkt z wykazu współrzędnych. `x` to północ, `y` to wschód.
#[derive(Debug, Clone, PartialEq)]
pub struct PointRecord {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub h: Option<f64>,
    /// Kod albo opis punktu; pusty, gdy go nie ma.
    pub code: String,
}

/// Kolejność współrzędnych w pliku.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnOrder {
    /// numer, X (północ), Y (wschód)
    NrXY,
    /// numer, Y (wschód), X (północ)
    NrYX,
}

/// Wiersz, którego nie udało się odczytać.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Numer wiersza w pliku, od 1.
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parsed {
    pub points: Vec<PointRecord>,
    pub errors: Vec<ParseError>,
}

fn split_fields(line: &str) -> Vec<&str> {
    if line.contains([';', '\t']) {
        line.split([';', '\t']).map(str::trim).collect()
    } else if line.split_whitespace().nth(2).is_some() || line.split(',').nth(2).is_none() {
        line.split_whitespace().collect()
    } else {
        line.split(',').map(str::trim).collect()
    }
}

fn number(field: &str) -> Option<f64> {
    field
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn parse_line(line: &str, order: ColumnOrder) -> Result<PointRecord, String> {
    let fields = split_fields(line);
    let [id, first, second, rest @ ..] = fields.as_slice() else {
        return Err("za mało pól (potrzebne: numer i dwie współrzędne)".into());
    };
    if id.is_empty() {
        return Err("pusty numer punktu".into());
    }
    let coord = |f: &str| number(f).ok_or_else(|| format!("„{f}” nie jest liczbą"));
    let (first, second) = (coord(first)?, coord(second)?);
    let (x, y) = match order {
        ColumnOrder::NrXY => (first, second),
        ColumnOrder::NrYX => (second, first),
    };
    // Czwarte pole to wysokość, jeśli jest liczbą albo jest puste; inaczej to już kod.
    let (h, code_fields) = match rest {
        [] => (None, rest),
        ["", tail @ ..] => (None, tail),
        [f, tail @ ..] => match number(f) {
            Some(h) => (Some(h), tail),
            None => (None, rest),
        },
    };
    Ok(PointRecord {
        id: id.to_string(),
        x,
        y,
        h,
        code: code_fields.join(" ").trim().to_string(),
    })
}

/// Odczytuje wykaz współrzędnych. Błędne wiersze trafiają do `errors`, reszta się wczytuje.
///
/// Puste wiersze i komentarze (`#`, `//`) są pomijane. Pierwszy wiersz z danymi,
/// jeśli nie da się go odczytać, jest traktowany jako nagłówek.
pub fn parse_points(text: &str, order: ColumnOrder) -> Parsed {
    let mut parsed = Parsed::default();
    let mut seen_content = false;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let first_content = !seen_content;
        seen_content = true;
        match parse_line(line, order) {
            Ok(p) => parsed.points.push(p),
            Err(_) if first_content => {}
            Err(message) => parsed.errors.push(ParseError {
                line: i + 1,
                message,
            }),
        }
    }
    parsed
}

/// Zapisuje wykaz współrzędnych rozdzielony tabulatorami, z kropką dziesiętną.
pub fn write_points(points: &[PointRecord], order: ColumnOrder, decimals: usize) -> String {
    let mut out = String::new();
    for p in points {
        let (first, second) = match order {
            ColumnOrder::NrXY => (p.x, p.y),
            ColumnOrder::NrYX => (p.y, p.x),
        };
        out.push_str(&format!(
            "{}\t{first:.decimals$}\t{second:.decimals$}",
            p.id
        ));
        match (p.h, p.code.is_empty()) {
            (Some(h), true) => out.push_str(&format!("\t{h:.decimals$}")),
            (Some(h), false) => out.push_str(&format!("\t{h:.decimals$}\t{}", p.code)),
            (None, false) => out.push_str(&format!("\t\t{}", p.code)),
            (None, true) => {}
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, x: f64, y: f64, h: Option<f64>, code: &str) -> PointRecord {
        PointRecord {
            id: id.into(),
            x,
            y,
            h,
            code: code.into(),
        }
    }

    #[test]
    fn separators_and_decimal_comma() {
        let text = "\
101 5788456.49 7500833.51
102\t5788460,10\t7500840,25\t112,34
103;5788470.00;7500850.00;;granica
104,5788480.5,7500860.5,110.0,słup betonowy
105  5788490,00   7500870,00   bud 1
";
        let parsed = parse_points(text, ColumnOrder::NrXY);
        assert_eq!(parsed.errors, vec![]);
        assert_eq!(
            parsed.points,
            vec![
                rec("101", 5788456.49, 7500833.51, None, ""),
                rec("102", 5788460.10, 7500840.25, Some(112.34), ""),
                rec("103", 5788470.0, 7500850.0, None, "granica"),
                rec("104", 5788480.5, 7500860.5, Some(110.0), "słup betonowy"),
                rec("105", 5788490.0, 7500870.0, None, "bud 1"),
            ]
        );
    }

    #[test]
    fn column_order_header_comments_and_errors() {
        let text =
            "\u{feff}Nr Y X H\n# komentarz\n\n1 7500833.51 5788456.49\n2 abc 5\n3 1\n// koniec\n";
        let parsed = parse_points(text, ColumnOrder::NrYX);
        assert_eq!(
            parsed.points,
            vec![rec("1", 5788456.49, 7500833.51, None, "")]
        );
        assert_eq!(parsed.errors.len(), 2);
        assert_eq!(parsed.errors[0].line, 5);
        assert_eq!(parsed.errors[1].line, 6);
    }

    #[test]
    fn write_then_parse_round_trip() {
        let points = vec![
            rec("A 1", 5788456.49, 7500833.51, None, ""),
            rec("2", 5788460.1, 7500840.25, Some(112.34), ""),
            rec("3", 5788470.0, 7500850.0, None, "granica działki"),
            rec("4", 5788480.5, 7500860.5, Some(110.0), "słup"),
        ];
        for order in [ColumnOrder::NrXY, ColumnOrder::NrYX] {
            let text = write_points(&points, order, 2);
            let parsed = parse_points(&text, order);
            assert_eq!(parsed.errors, vec![]);
            assert_eq!(parsed.points, points);
        }
        assert_eq!(
            write_points(&points[1..2], ColumnOrder::NrXY, 2),
            "2\t5788460.10\t7500840.25\t112.34\n"
        );
    }
}
