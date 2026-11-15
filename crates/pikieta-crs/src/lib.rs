//! Polskie układy współrzędnych płaskich na elipsoidzie GRS80:
//! PL-2000 (EPSG:2176–2179), PL-1992 (EPSG:2180) i UTM (EPSG:25833–25835).
//!
//! Wszystkie są odwzorowaniem Gaussa-Krügera i różnią się tylko parametrami,
//! więc przeliczenie między nimi idzie przez współrzędne geodezyjne bez zmiany
//! układu odniesienia.

mod tm;

use pikieta_core::Point2;
pub use tm::TransverseMercator;

/// Współrzędne geodezyjne na GRS80, w stopniach dziesiętnych.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geodetic {
    /// Szerokość (B), dodatnia na północ.
    pub lat: f64,
    /// Długość (L), dodatnia na wschód.
    pub lon: f64,
}

impl Geodetic {
    pub const fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }
}

/// Układ współrzędnych płaskich.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Crs {
    /// PL-2000, strefy 5–8 (południki osiowe 15°, 18°, 21°, 24°).
    Pl2000 { zone: u8 },
    /// PL-1992, jedna strefa dla całego kraju.
    Pl1992,
    /// UTM, strefy 33–35.
    Utm { zone: u8 },
}

impl Crs {
    /// Wszystkie obsługiwane układy.
    pub const ALL: [Crs; 8] = [
        Crs::Pl2000 { zone: 5 },
        Crs::Pl2000 { zone: 6 },
        Crs::Pl2000 { zone: 7 },
        Crs::Pl2000 { zone: 8 },
        Crs::Pl1992,
        Crs::Utm { zone: 33 },
        Crs::Utm { zone: 34 },
        Crs::Utm { zone: 35 },
    ];

    /// Układ o podanym kodzie EPSG albo `None`, gdy nie jest obsługiwany.
    pub fn from_epsg(code: u32) -> Option<Crs> {
        match code {
            2176..=2179 => Some(Crs::Pl2000 {
                zone: (code - 2171) as u8,
            }),
            2180 => Some(Crs::Pl1992),
            25833..=25835 => Some(Crs::Utm {
                zone: (code - 25800) as u8,
            }),
            _ => None,
        }
    }

    pub fn epsg(self) -> u32 {
        match self {
            Crs::Pl2000 { zone } => 2171 + u32::from(zone),
            Crs::Pl1992 => 2180,
            Crs::Utm { zone } => 25800 + u32::from(zone),
        }
    }

    /// Strefa PL-2000 właściwa dla długości geodezyjnej (w stopniach).
    pub fn pl2000_for_longitude(lon: f64) -> Crs {
        let zone = (lon / 3.0).round().clamp(5.0, 8.0) as u8;
        Crs::Pl2000 { zone }
    }

    /// Parametry odwzorowania.
    pub fn projection(self) -> TransverseMercator {
        match self {
            Crs::Pl2000 { zone } => {
                let z = f64::from(zone);
                TransverseMercator::grs80(3.0 * z, 0.999923, z * 1_000_000.0 + 500_000.0, 0.0)
            }
            Crs::Pl1992 => TransverseMercator::grs80(19.0, 0.9993, 500_000.0, -5_300_000.0),
            Crs::Utm { zone } => {
                let lon0 = 6.0 * f64::from(zone) - 183.0;
                TransverseMercator::grs80(lon0, 0.9996, 500_000.0, 0.0)
            }
        }
    }

    /// Współrzędne geodezyjne na płaskie.
    pub fn forward(self, g: Geodetic) -> Point2 {
        self.projection().forward(g)
    }

    /// Współrzędne płaskie na geodezyjne.
    pub fn inverse(self, p: Point2) -> Geodetic {
        self.projection().inverse(p)
    }
}

/// Przelicza punkt między dwoma układami płaskimi.
pub fn transform(p: Point2, from: Crs, to: Crs) -> Point2 {
    if from == to {
        return p;
    }
    to.forward(from.inverse(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (EPSG, długość, szerokość, wschód, północ) z programu `proj` (PROJ, +ellps=GRS80).
    /// Część punktów leży celowo daleko poza strefą.
    const REFERENCE: &[(u32, f64, f64, f64, f64)] = &[
        (2176, 14.25, 49.1, 5445235.54310, 5440600.69483),
        (2176, 15.0, 52.0, 5500000.00000, 5762899.77243),
        (2176, 16.4, 54.7, 5590249.07311, 6064267.11706),
        (2176, 24.1, 53.9, 6097346.29372, 6012791.37903),
        (2177, 14.25, 49.1, 6226204.38477, 5447106.68142),
        (2177, 15.0, 52.0, 6294004.53939, 5767151.18801),
        (2177, 16.4, 54.7, 6396859.24410, 6064542.62960),
        (2178, 21.0122, 52.2297, 7500833.51239, 5788456.48654),
        (2178, 22.5, 49.3, 7609086.73257, 5463653.46145),
        (2178, 24.1, 53.9, 7703725.74586, 5978780.49429),
        (2179, 24.1, 53.9, 8506572.77639, 5974330.19718),
        (2179, 23.99, 51.25, 8499301.85608, 5679461.08502),
        (2179, 17.03, 51.11, 8012173.97924, 5687033.43389),
        (2180, 14.25, 49.1, 153429.71543, 147810.29757),
        (2180, 15.0, 52.0, 225534.13777, 466864.29239),
        (2180, 16.4, 54.7, 332512.54939, 762691.75188),
        (2180, 18.6, 54.35, 474008.48612, 720730.23242),
        (2180, 19.0, 50.06, 500000.00000, 243637.55551),
        (2180, 21.0122, 52.2297, 637382.20444, 486757.20948),
        (2180, 22.5, 49.3, 754357.87523, 165060.62207),
        (2180, 24.1, 53.9, 834867.38197, 682658.73702),
        (25834, 19.0, 50.06, 356849.39374, 5547217.77223),
        (25834, 21.0122, 52.2297, 500833.24315, 5786586.67112),
        (25834, 22.5, 49.3, 609051.49484, 5461888.56548),
    ];

    /// 0,02 mm: dokładność zapisu wartości referencyjnych (5 miejsc) z zapasem.
    const TOL_M: f64 = 2e-5;

    #[test]
    fn forward_matches_proj() {
        for &(epsg, lon, lat, east, north) in REFERENCE {
            let p = Crs::from_epsg(epsg)
                .unwrap()
                .forward(Geodetic::new(lat, lon));
            assert!(
                (p.x - north).abs() < TOL_M && (p.y - east).abs() < TOL_M,
                "EPSG:{epsg} ({lat}, {lon}): {p:?}, oczekiwano x={north} y={east}"
            );
        }
    }

    #[test]
    fn inverse_matches_proj() {
        // 1e-10° to około 0,01 mm na powierzchni Ziemi.
        for &(epsg, lon, lat, east, north) in REFERENCE {
            let g = Crs::from_epsg(epsg)
                .unwrap()
                .inverse(Point2::new(north, east));
            assert!(
                (g.lat - lat).abs() < 3e-10 && (g.lon - lon).abs() < 3e-10,
                "EPSG:{epsg}: {g:?}, oczekiwano ({lat}, {lon})"
            );
        }
    }

    #[test]
    fn transform_between_systems_matches_proj() {
        // Warszawa: PL-2000 strefa 7 → PL-1992.
        let p = transform(
            Point2::new(5788456.48654, 7500833.51239),
            Crs::Pl2000 { zone: 7 },
            Crs::Pl1992,
        );
        assert!((p.x - 486757.20948).abs() < 5e-5, "{p:?}");
        assert!((p.y - 637382.20444).abs() < 5e-5, "{p:?}");
    }

    #[test]
    fn round_trip_all_systems() {
        for crs in Crs::ALL {
            let g = Geodetic::new(51.7, 19.4);
            let back = crs.inverse(crs.forward(g));
            assert!((back.lat - g.lat).abs() < 1e-10, "{crs:?}");
            assert!((back.lon - g.lon).abs() < 1e-10, "{crs:?}");
        }
    }

    #[test]
    fn epsg_codes() {
        for crs in Crs::ALL {
            assert_eq!(Crs::from_epsg(crs.epsg()), Some(crs));
        }
        assert_eq!(Crs::from_epsg(4326), None);
        assert_eq!(Crs::from_epsg(2180), Some(Crs::Pl1992));
        assert_eq!(Crs::Utm { zone: 34 }.projection().lon0_deg(), 21.0);
    }

    #[test]
    fn zone_for_longitude() {
        assert_eq!(Crs::pl2000_for_longitude(14.2), Crs::Pl2000 { zone: 5 });
        assert_eq!(Crs::pl2000_for_longitude(16.6), Crs::Pl2000 { zone: 6 });
        assert_eq!(Crs::pl2000_for_longitude(21.0), Crs::Pl2000 { zone: 7 });
        assert_eq!(Crs::pl2000_for_longitude(24.1), Crs::Pl2000 { zone: 8 });
    }
}
