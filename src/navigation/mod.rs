pub mod pseudorange;
pub mod position_solver;
pub mod corrections;

pub use pseudorange::Pseudorange;
pub use position_solver::{PositionSolver, PositionSolution};
pub use corrections::{ionospheric_correction, tropospheric_correction};

use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnssPosition {
    pub latitude: f64,   // degrees
    pub longitude: f64,  // degrees
    pub altitude: f64,   // meters above WGS84 ellipsoid
    pub time: f64,       // GPS time (seconds)
    pub clock_bias: f64, // receiver clock bias (meters)

    // Accuracy estimates
    pub hdop: f64,       // Horizontal dilution of precision
    pub vdop: f64,       // Vertical dilution of precision
    pub pdop: f64,       // Position dilution of precision

    // Number of satellites used
    pub num_satellites: usize,
}

impl GnssPosition {
    /// Convert ECEF position to latitude, longitude, altitude
    pub fn from_ecef(ecef: Vector3<f64>, clock_bias: f64, num_satellites: usize) -> Self {
        let (lat, lon, alt) = ecef_to_lla(ecef);

        Self {
            latitude: lat,
            longitude: lon,
            altitude: alt,
            time: 0.0,
            clock_bias,
            hdop: 0.0,
            vdop: 0.0,
            pdop: 0.0,
            num_satellites,
        }
    }

    /// Get position as ECEF vector
    pub fn to_ecef(&self) -> Vector3<f64> {
        lla_to_ecef(self.latitude, self.longitude, self.altitude)
    }
}

/// Convert ECEF (Earth-Centered Earth-Fixed) to LLA (Latitude, Longitude, Altitude)
pub fn ecef_to_lla(ecef: Vector3<f64>) -> (f64, f64, f64) {
    use crate::constants::{EARTH_A, EARTH_E2};

    let x = ecef.x;
    let y = ecef.y;
    let z = ecef.z;

    // Longitude
    let lon = y.atan2(x);

    // Latitude (iterative)
    let p = (x * x + y * y).sqrt();
    let mut lat = (z / p).atan();

    for _ in 0..5 {
        let n = EARTH_A / (1.0 - EARTH_E2 * lat.sin() * lat.sin()).sqrt();
        lat = (z / p + EARTH_E2 * n * lat.sin() / p).atan();
    }

    // Altitude
    let n = EARTH_A / (1.0 - EARTH_E2 * lat.sin() * lat.sin()).sqrt();
    let alt = p / lat.cos() - n;

    (lat.to_degrees(), lon.to_degrees(), alt)
}

/// Convert LLA to ECEF
pub fn lla_to_ecef(lat_deg: f64, lon_deg: f64, alt: f64) -> Vector3<f64> {
    use crate::constants::{EARTH_A, EARTH_E2};

    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();

    let n = EARTH_A / (1.0 - EARTH_E2 * lat.sin() * lat.sin()).sqrt();

    let x = (n + alt) * lat.cos() * lon.cos();
    let y = (n + alt) * lat.cos() * lon.sin();
    let z = (n * (1.0 - EARTH_E2) + alt) * lat.sin();

    Vector3::new(x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lla_to_ecef_conversion() {
        // Test with a known location (equator, prime meridian, sea level)
        let ecef = lla_to_ecef(0.0, 0.0, 0.0);
        let (lat, lon, alt) = ecef_to_lla(ecef);

        assert!((lat - 0.0).abs() < 1e-6);
        assert!((lon - 0.0).abs() < 1e-6);
        assert!((alt - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_ecef_magnitude() {
        // At equator, ECEF magnitude should be close to Earth radius
        let ecef = lla_to_ecef(0.0, 0.0, 0.0);
        let magnitude = (ecef.x * ecef.x + ecef.y * ecef.y + ecef.z * ecef.z).sqrt();
        assert!((magnitude - 6_378_137.0).abs() < 1000.0);
    }
}
