use crate::constants::*;
use nalgebra::{Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ephemeris {
    pub prn: u8,
    pub week: u16,
    pub toe: f64,         // Time of ephemeris (seconds)
    pub toc: f64,         // Time of clock (seconds)

    // Clock correction parameters
    pub af0: f64,         // Clock bias (seconds)
    pub af1: f64,         // Clock drift (sec/sec)
    pub af2: f64,         // Clock drift rate (sec/sec^2)
    pub tgd: f64,         // Group delay (seconds)

    // Orbit parameters
    pub m0: f64,          // Mean anomaly at reference time (radians)
    pub delta_n: f64,     // Mean motion difference (radians/sec)
    pub e: f64,           // Eccentricity
    pub sqrt_a: f64,      // Square root of semi-major axis (sqrt(m))

    pub omega0: f64,      // Longitude of ascending node (radians)
    pub i0: f64,          // Inclination angle at reference time (radians)
    pub omega: f64,       // Argument of perigee (radians)
    pub omega_dot: f64,   // Rate of right ascension (radians/sec)
    pub idot: f64,        // Rate of inclination angle (radians/sec)

    pub cuc: f64,         // Cosine correction to argument of latitude
    pub cus: f64,         // Sine correction to argument of latitude
    pub crc: f64,         // Cosine correction to orbit radius
    pub crs: f64,         // Sine correction to orbit radius
    pub cic: f64,         // Cosine correction to inclination
    pub cis: f64,         // Sine correction to inclination

    // Status
    pub iode: u8,         // Issue of data (ephemeris)
    pub iodc: u16,        // Issue of data (clock)
    pub health: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Almanac {
    pub prn: u8,
    pub week: u16,
    pub toa: f64,         // Time of almanac (seconds)
    pub e: f64,           // Eccentricity
    pub sqrt_a: f64,      // Square root of semi-major axis
    pub omega0: f64,      // Longitude of ascending node
    pub i0: f64,          // Inclination angle
    pub omega: f64,       // Argument of perigee
    pub m0: f64,          // Mean anomaly
    pub af0: f64,         // Clock bias
    pub af1: f64,         // Clock drift
    pub health: u8,
}

impl Ephemeris {
    /// Parse ephemeris from navigation message subframes 1-3
    pub fn from_subframes(prn: u8, subframes: &[u32]) -> Option<Self> {
        if subframes.len() < 30 {
            return None;
        }

        // This is a simplified parser - in production, you'd extract all fields
        // according to GPS ICD-200
        Some(Self {
            prn,
            week: 0,  // Extract from subframe 1
            toe: 0.0,
            toc: 0.0,
            af0: 0.0,
            af1: 0.0,
            af2: 0.0,
            tgd: 0.0,
            m0: 0.0,
            delta_n: 0.0,
            e: 0.0,
            sqrt_a: 0.0,
            omega0: 0.0,
            i0: 0.0,
            omega: 0.0,
            omega_dot: 0.0,
            idot: 0.0,
            cuc: 0.0,
            cus: 0.0,
            crc: 0.0,
            crs: 0.0,
            cic: 0.0,
            cis: 0.0,
            iode: 0,
            iodc: 0,
            health: 0,
        })
    }

    /// Calculate satellite position at given GPS time
    pub fn satellite_position(&self, gps_time: f64) -> Vector3<f64> {
        let tk = gps_time - self.toe;

        // Semi-major axis
        let a = self.sqrt_a * self.sqrt_a;

        // Computed mean motion
        let n0 = (EARTH_GM / (a * a * a)).sqrt();
        let n = n0 + self.delta_n;

        // Mean anomaly
        let mk = self.m0 + n * tk;

        // Solve Kepler's equation for eccentric anomaly (iterative)
        let mut ek = mk;
        for _ in 0..10 {
            ek = mk + self.e * ek.sin();
        }

        // True anomaly
        let sin_vk = ((1.0 - self.e * self.e).sqrt() * ek.sin()) / (1.0 - self.e * ek.cos());
        let cos_vk = (ek.cos() - self.e) / (1.0 - self.e * ek.cos());
        let vk = sin_vk.atan2(cos_vk);

        // Argument of latitude
        let phi_k = vk + self.omega;

        // Second harmonic perturbations
        let delta_uk = self.cus * (2.0 * phi_k).sin() + self.cuc * (2.0 * phi_k).cos();
        let delta_rk = self.crs * (2.0 * phi_k).sin() + self.crc * (2.0 * phi_k).cos();
        let delta_ik = self.cis * (2.0 * phi_k).sin() + self.cic * (2.0 * phi_k).cos();

        // Corrected argument of latitude
        let uk = phi_k + delta_uk;

        // Corrected radius
        let rk = a * (1.0 - self.e * ek.cos()) + delta_rk;

        // Corrected inclination
        let ik = self.i0 + delta_ik + self.idot * tk;

        // Positions in orbital plane
        let xk_prime = rk * uk.cos();
        let yk_prime = rk * uk.sin();

        // Corrected longitude of ascending node
        let omega_k = self.omega0 + (self.omega_dot - EARTH_OMEGA_DOT) * tk
            - EARTH_OMEGA_DOT * self.toe;

        // Earth-fixed coordinates
        let xk = xk_prime * omega_k.cos() - yk_prime * ik.cos() * omega_k.sin();
        let yk = xk_prime * omega_k.sin() + yk_prime * ik.cos() * omega_k.cos();
        let zk = yk_prime * ik.sin();

        Vector3::new(xk, yk, zk)
    }

    /// Calculate satellite clock correction
    pub fn clock_correction(&self, gps_time: f64) -> f64 {
        let dt = gps_time - self.toc;
        self.af0 + self.af1 * dt + self.af2 * dt * dt - self.tgd
    }

    /// Calculate satellite velocity
    pub fn satellite_velocity(&self, gps_time: f64) -> Vector3<f64> {
        // Numerical derivative (simple approximation)
        let dt = 0.1; // 100ms step
        let pos1 = self.satellite_position(gps_time - dt / 2.0);
        let pos2 = self.satellite_position(gps_time + dt / 2.0);

        (pos2 - pos1) / dt
    }
}

impl Almanac {
    /// Parse almanac from subframe 4 or 5
    pub fn from_subframe(prn: u8, subframe: &[u32]) -> Option<Self> {
        if subframe.len() < 10 {
            return None;
        }

        // Simplified parser
        Some(Self {
            prn,
            week: 0,
            toa: 0.0,
            e: 0.0,
            sqrt_a: 0.0,
            omega0: 0.0,
            i0: 0.0,
            omega: 0.0,
            m0: 0.0,
            af0: 0.0,
            af1: 0.0,
            health: 0,
        })
    }

    /// Calculate approximate satellite position (lower accuracy than ephemeris)
    pub fn satellite_position(&self, gps_time: f64) -> Vector3<f64> {
        // Simplified calculation - similar to ephemeris but with fewer corrections
        let tk = gps_time - self.toa;
        let a = self.sqrt_a * self.sqrt_a;
        let n0 = (EARTH_GM / (a * a * a)).sqrt();
        let mk = self.m0 + n0 * tk;

        // Simplified Kepler solution
        let mut ek = mk;
        for _ in 0..5 {
            ek = mk + self.e * ek.sin();
        }

        let vk = 2.0 * ((((1.0 + self.e) / (1.0 - self.e)).sqrt() * (ek / 2.0).tan()).atan());
        let phi_k = vk + self.omega;
        let rk = a * (1.0 - self.e * ek.cos());

        let xk_prime = rk * phi_k.cos();
        let yk_prime = rk * phi_k.sin();

        let omega_k = self.omega0 + (0.0 - EARTH_OMEGA_DOT) * tk - EARTH_OMEGA_DOT * self.toa;

        let xk = xk_prime * omega_k.cos() - yk_prime * self.i0.cos() * omega_k.sin();
        let yk = xk_prime * omega_k.sin() + yk_prime * self.i0.cos() * omega_k.cos();
        let zk = yk_prime * self.i0.sin();

        Vector3::new(xk, yk, zk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeris_creation() {
        let eph = Ephemeris {
            prn: 1,
            week: 2000,
            toe: 0.0,
            toc: 0.0,
            af0: 0.0,
            af1: 0.0,
            af2: 0.0,
            tgd: 0.0,
            m0: 0.0,
            delta_n: 0.0,
            e: 0.01,
            sqrt_a: 5153.5,
            omega0: 0.0,
            i0: 0.96,
            omega: 0.0,
            omega_dot: 0.0,
            idot: 0.0,
            cuc: 0.0,
            cus: 0.0,
            crc: 0.0,
            crs: 0.0,
            cic: 0.0,
            cis: 0.0,
            iode: 0,
            iodc: 0,
            health: 0,
        };

        let pos = eph.satellite_position(0.0);
        // Check that position is roughly at GPS orbit radius (26000 km)
        let radius = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
        assert!(radius > 20_000_000.0 && radius < 30_000_000.0);
    }
}
