//! Odwzorowanie Gaussa-Krügera (poprzeczne Merkatora) szeregami Krügera
//! względem trzeciego spłaszczenia `n`, do wyrazów `n⁴`.
//!
//! Wzory: L. Krüger (1912), w postaci podanej przez C. Karneya,
//! „Transverse Mercator with an accuracy of a few nanometers" (2011).
//! Testy porównują wyniki z PROJ do 9° od południka osiowego z tolerancją 0,02 mm.

use crate::Geodetic;
use pikieta_core::Point2;

const GRS80_A: f64 = 6_378_137.0;
const GRS80_INV_F: f64 = 298.257_222_101;

/// Parametry odwzorowania wraz z wyliczonymi stałymi elipsoidy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransverseMercator {
    lon0: f64,
    false_easting: f64,
    false_northing: f64,
    /// Pierwszy mimośród.
    e: f64,
    /// Skala na południku osiowym razy promień prostujący.
    k0_a: f64,
    alpha: [f64; 4],
    beta: [f64; 4],
}

impl TransverseMercator {
    /// Odwzorowanie na elipsoidzie GRS80 z szerokością początkową 0°.
    pub fn grs80(lon0_deg: f64, k0: f64, false_easting: f64, false_northing: f64) -> Self {
        let f = 1.0 / GRS80_INV_F;
        let n = f / (2.0 - f);
        let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
        let rectifying_radius = GRS80_A / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0);
        Self {
            lon0: lon0_deg.to_radians(),
            false_easting,
            false_northing,
            e: (f * (2.0 - f)).sqrt(),
            k0_a: k0 * rectifying_radius,
            alpha: [
                n / 2.0 - 2.0 / 3.0 * n2 + 5.0 / 16.0 * n3 + 41.0 / 180.0 * n4,
                13.0 / 48.0 * n2 - 3.0 / 5.0 * n3 + 557.0 / 1440.0 * n4,
                61.0 / 240.0 * n3 - 103.0 / 140.0 * n4,
                49561.0 / 161280.0 * n4,
            ],
            beta: [
                n / 2.0 - 2.0 / 3.0 * n2 + 37.0 / 96.0 * n3 - 1.0 / 360.0 * n4,
                1.0 / 48.0 * n2 + 1.0 / 15.0 * n3 - 437.0 / 1440.0 * n4,
                17.0 / 480.0 * n3 - 37.0 / 840.0 * n4,
                4397.0 / 161280.0 * n4,
            ],
        }
    }

    /// Południk osiowy w stopniach.
    pub fn lon0_deg(&self) -> f64 {
        self.lon0.to_degrees()
    }

    pub fn forward(&self, g: Geodetic) -> Point2 {
        let phi = g.lat.to_radians();
        let dl = g.lon.to_radians() - self.lon0;

        // Tangens szerokości konforemnej.
        let sin_phi = phi.sin();
        let t = (sin_phi.atanh() - self.e * (self.e * sin_phi).atanh()).sinh();
        let xi0 = t.atan2(dl.cos());
        let eta0 = (dl.sin() / t.hypot(1.0)).atanh();

        let (mut xi, mut eta) = (xi0, eta0);
        for (j, a) in self.alpha.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi += a * (k * xi0).sin() * (k * eta0).cosh();
            eta += a * (k * xi0).cos() * (k * eta0).sinh();
        }

        Point2::new(
            self.k0_a * xi + self.false_northing,
            self.k0_a * eta + self.false_easting,
        )
    }

    pub fn inverse(&self, p: Point2) -> Geodetic {
        let xi = (p.x - self.false_northing) / self.k0_a;
        let eta = (p.y - self.false_easting) / self.k0_a;

        let (mut xi0, mut eta0) = (xi, eta);
        for (j, b) in self.beta.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi0 -= b * (k * xi).sin() * (k * eta).cosh();
            eta0 -= b * (k * xi).cos() * (k * eta).sinh();
        }

        // Tangens szerokości konforemnej, potem geodezyjnej metodą Newtona.
        let t_conf = xi0.sin() / eta0.sinh().hypot(xi0.cos());
        let e2 = self.e * self.e;
        let mut tau = t_conf;
        for _ in 0..8 {
            let sigma = (self.e * (self.e * tau / tau.hypot(1.0)).atanh()).sinh();
            let t_i = tau * sigma.hypot(1.0) - sigma * tau.hypot(1.0);
            let d_tau = (t_conf - t_i) / t_i.hypot(1.0) * (1.0 + (1.0 - e2) * tau * tau)
                / ((1.0 - e2) * tau.hypot(1.0));
            tau += d_tau;
            if d_tau.abs() < 1e-15 * tau.abs().max(1.0) {
                break;
            }
        }

        Geodetic::new(
            tau.atan().to_degrees(),
            (self.lon0 + eta0.sinh().atan2(xi0.cos())).to_degrees(),
        )
    }
}
