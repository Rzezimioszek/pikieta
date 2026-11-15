//! `pikieta`: transformacje układów i obliczenia geodezyjne z wiersza poleceń.

use pikieta_calc::{area, azimuth_distance};
use pikieta_core::{rad_to_grad, Point2};
use pikieta_crs::{transform, Crs, Geodetic};
use std::process::ExitCode;

const USAGE: &str = "\
Użycie:
  pikieta transformuj <z EPSG> <do EPSG> <X> <Y>   przeliczenie między układami płaskimi
  pikieta xy <EPSG> <B> <L>                        geodezyjne (stopnie) na płaskie
  pikieta bl <EPSG> <X> <Y>                        płaskie na geodezyjne (stopnie)
  pikieta azymut <Xa> <Ya> <Xb> <Yb>               azymut (grady) i długość
  pikieta pole <X1> <Y1> <X2> <Y2> <X3> <Y3> ...   pole wieloboku ze współrzędnych
  pikieta skala <EPSG> <X> <Y>                     skala odwzorowania i zbieżność południków (grady)
  pikieta redukcja <EPSG> <Xa> <Ya> <Xb> <Yb>      długość z XY, skala odcinka, długość na elipsoidzie
  pikieta pole-elipsoida <EPSG> <X1> <Y1> ...      pole wieloboku zredukowane na elipsoidę

X to północ, Y to wschód. Układy: 2176-2179 (PL-2000), 2180 (PL-1992), 25833-25835 (UTM).
Liczby można pisać z kropką albo przecinkiem.";

fn number(s: &str) -> Result<f64, String> {
    s.replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("„{s}” nie jest liczbą"))
}

fn crs(s: &str) -> Result<Crs, String> {
    let code = s
        .strip_prefix("EPSG:")
        .or_else(|| s.strip_prefix("epsg:"))
        .unwrap_or(s);
    code.parse()
        .ok()
        .and_then(Crs::from_epsg)
        .ok_or_else(|| format!("nieobsługiwany układ „{s}”"))
}

fn numbers(args: &[String]) -> Result<Vec<f64>, String> {
    args.iter().map(|a| number(a)).collect()
}

fn run(args: &[String]) -> Result<String, String> {
    let (command, rest) = args.split_first().ok_or("brak polecenia")?;
    match (command.as_str(), rest) {
        ("transformuj", [from, to, x, y]) => {
            let p = transform(Point2::new(number(x)?, number(y)?), crs(from)?, crs(to)?);
            Ok(format!("{:.4} {:.4}", p.x, p.y))
        }
        ("xy", [epsg, lat, lon]) => {
            let p = crs(epsg)?.forward(Geodetic::new(number(lat)?, number(lon)?));
            Ok(format!("{:.4} {:.4}", p.x, p.y))
        }
        ("bl", [epsg, x, y]) => {
            let g = crs(epsg)?.inverse(Point2::new(number(x)?, number(y)?));
            Ok(format!("{:.9} {:.9}", g.lat, g.lon))
        }
        ("azymut", coords @ [_, _, _, _]) => {
            let c = numbers(coords)?;
            let (az, d) = azimuth_distance(Point2::new(c[0], c[1]), Point2::new(c[2], c[3]))
                .ok_or("punkty się pokrywają, azymut jest nieokreślony")?;
            Ok(format!("{:.4} {:.4}", rad_to_grad(az), d))
        }
        ("skala", [epsg, x, y]) => {
            let (crs, p) = (crs(epsg)?, Point2::new(number(x)?, number(y)?));
            Ok(format!(
                "{:.8} {:.5}",
                crs.scale(p),
                rad_to_grad(crs.convergence(p))
            ))
        }
        ("redukcja", [epsg, coords @ ..]) if coords.len() == 4 => {
            let (crs, c) = (crs(epsg)?, numbers(coords)?);
            let (a, b) = (Point2::new(c[0], c[1]), Point2::new(c[2], c[3]));
            let (_, d) = azimuth_distance(a, b).ok_or("punkty się pokrywają")?;
            Ok(format!(
                "{:.4} {:.8} {:.4}",
                d,
                crs.line_scale(a, b),
                crs.ellipsoidal_distance(a, b)
            ))
        }
        ("pole-elipsoida", [epsg, coords @ ..]) if coords.len() >= 6 && coords.len() % 2 == 0 => {
            let points: Vec<Point2> = numbers(coords)?
                .chunks(2)
                .map(|c| Point2::new(c[0], c[1]))
                .collect();
            Ok(format!("{:.4}", crs(epsg)?.ellipsoidal_area(&points)))
        }
        ("pole", coords) if coords.len() >= 6 && coords.len() % 2 == 0 => {
            let points: Vec<Point2> = numbers(coords)?
                .chunks(2)
                .map(|c| Point2::new(c[0], c[1]))
                .collect();
            Ok(format!("{:.4}", area(&points)))
        }
        _ => Err(format!(
            "nieznane polecenie albo zła liczba argumentów: {command}"
        )),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "pomoc") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match run(&args) {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Błąd: {e}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    fn call(line: &str) -> Result<String, String> {
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        run(&args)
    }

    #[test]
    fn commands() {
        assert_eq!(
            call("transformuj 2178 EPSG:2180 5788456.48654 7500833.51239").unwrap(),
            "486757.2095 637382.2044"
        );
        assert_eq!(
            call("xy 2178 52,2297 21,0122").unwrap(),
            "5788456.4865 7500833.5124"
        );
        assert_eq!(
            call("bl 2178 5788456.48654 7500833.51239").unwrap(),
            "52.229700000 21.012200000"
        );
        assert_eq!(call("azymut 0 0 100 100").unwrap(), "50.0000 141.4214");
        assert_eq!(call("pole 0 0 100 0 100 100 0 100").unwrap(), "10000.0000");
        // Warszawa w strefie 7: skala i zbieżność (0,00964377° = 0,01072g) jak w PROJ.
        assert_eq!(
            call("skala 2178 5788456.48654 7500833.51239").unwrap(),
            "0.99992301 0.01072"
        );
        assert_eq!(
            call("redukcja 2178 5788456.49 7500833.51 5788456.49 7501833.51").unwrap(),
            "1000.0000 0.99992302 1000.0770"
        );
        assert_eq!(
            call("pole-elipsoida 2178 5788400 7500800 5788500 7500800 5788500 7500900 5788400 7500900")
                .unwrap(),
            "10001.5400"
        );
    }

    #[test]
    fn errors() {
        assert!(call("transformuj 4326 2180 1 2").is_err());
        assert!(call("azymut 1 1 1 1").is_err());
        assert!(call("azymut 1 1 x 1").is_err());
        assert!(call("pole 0 0 1 1").is_err());
        assert!(call("cokolwiek").is_err());
    }
}
