use crate::constants::*;
use nalgebra::Vector3;

/// GPS ionospheric model parameters (broadcast in subframe 4)
#[derive(Debug, Clone, Copy)]
pub struct IonoParams {
    pub alpha0: f64,
    pub alpha1: f64,
    pub alpha2: f64,
    pub alpha3: f64,
    pub beta0: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub beta3: f64,
}

impl Default for IonoParams {
    fn default() -> Self {
        Self {
            alpha0: 0.0,
            alpha1: 0.0,
            alpha2: 0.0,
            alpha3: 0.0,
            beta0: 0.0,
            beta1: 0.0,
            beta2: 0.0,
            beta3: 0.0,
        }
    }
}

/// Calculate ionospheric delay using Klobuchar model
pub fn ionospheric_correction(
    params: &IonoParams,
    user_pos: Vector3<f64>,
    sat_pos: Vector3<f64>,
    gps_time: f64,
) -> f64 {
    // Convert user position to geodetic
    let (lat_deg, lon_deg, _alt) = crate::navigation::ecef_to_lla(user_pos);
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();

    // Calculate elevation and azimuth to satellite
    let (el, az) = calculate_el_az(user_pos, sat_pos);

    if el < 0.0 {
        return 0.0; // Satellite below horizon
    }

    // Earth-centered angle (semi-circles)
    let psi = 0.0137 / (el / std::f64::consts::PI + 0.11) - 0.022;

    // Subionospheric latitude (semi-circles)
    let phi_i = (lat / std::f64::consts::PI) + psi * az.cos();
    let phi_i = phi_i.max(-0.416).min(0.416);

    // Subionospheric longitude (semi-circles)
    let lambda_i = (lon / std::f64::consts::PI) + psi * az.sin() / phi_i.cos();

    // Geomagnetic latitude (semi-circles)
    let phi_m = phi_i + 0.064 * ((lambda_i - 1.617) * std::f64::consts::PI).cos();

    // Local time (seconds)
    let t = 43200.0 * lambda_i + gps_time;
    let t = t % 86400.0;

    // Amplitude of ionospheric delay (seconds)
    let amp = params.alpha0 + params.alpha1 * phi_m + params.alpha2 * phi_m.powi(2)
        + params.alpha3 * phi_m.powi(3);
    let amp = amp.max(0.0);

    // Period of ionospheric delay (seconds)
    let per = params.beta0 + params.beta1 * phi_m + params.beta2 * phi_m.powi(2)
        + params.beta3 * phi_m.powi(3);
    let per = per.max(72000.0);

    // Phase (radians)
    let x = 2.0 * std::f64::consts::PI * (t - 50400.0) / per;

    // Slant factor
    let f = 1.0 + 16.0 * (0.53 - el / std::f64::consts::PI).powi(3);

    // Ionospheric time delay (seconds)
    let t_iono = if x.abs() < 1.57 {
        f * (5.0e-9 + amp * (1.0 - x.powi(2) / 2.0 + x.powi(4) / 24.0))
    } else {
        f * 5.0e-9
    };

    // Convert to meters (L1 frequency)
    t_iono * SPEED_OF_LIGHT
}

/// Calculate tropospheric delay using simplified Saastamoinen model
pub fn tropospheric_correction(
    user_pos: Vector3<f64>,
    sat_pos: Vector3<f64>,
    temperature_k: f64,
    pressure_mbar: f64,
    humidity_percent: f64,
) -> f64 {
    let (_lat, _lon, height_m) = crate::navigation::ecef_to_lla(user_pos);
    let (el, _az) = calculate_el_az(user_pos, sat_pos);

    if el < 0.0 {
        return 0.0;
    }

    // Saastamoinen model
    let el_deg = el.to_degrees();

    // Dry component
    let p0 = pressure_mbar * (1.0 - 0.0065 * height_m / temperature_k).powf(5.257);
    let dry = 0.002277 * p0 / (1.0 - 0.00266 * (2.0 * user_pos.z / EARTH_A).cos() - 0.00028 * height_m / 1000.0);

    // Wet component
    let e = humidity_percent / 100.0 * 6.11 * (17.27 * (temperature_k - 273.15) / (temperature_k - 35.85)).exp();
    let wet = 0.002277 * (1255.0 / temperature_k + 0.05) * e;

    // Mapping function (simplified)
    let mapping = 1.001 / (0.002001 + (el_deg.to_radians()).sin().powi(2)).sqrt();

    (dry + wet) * mapping
}

/// Calculate elevation and azimuth angles to satellite
fn calculate_el_az(user_pos: Vector3<f64>, sat_pos: Vector3<f64>) -> (f64, f64) {
    // Convert to ENU (East-North-Up) coordinates
    let (lat_deg, lon_deg, _) = crate::navigation::ecef_to_lla(user_pos);
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();

    // Vector from user to satellite
    let los = sat_pos - user_pos;

    // Rotation matrix ECEF to ENU
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    let e = -sin_lon * los.x + cos_lon * los.y;
    let n = -sin_lat * cos_lon * los.x - sin_lat * sin_lon * los.y + cos_lat * los.z;
    let u = cos_lat * cos_lon * los.x + cos_lat * sin_lon * los.y + sin_lat * los.z;

    // Elevation and azimuth
    let range = (e * e + n * n + u * u).sqrt();
    let elevation = (u / range).asin();
    let azimuth = e.atan2(n);

    (elevation, azimuth)
}

/// Calculate elevation angle in degrees
pub fn elevation_deg(user_pos: Vector3<f64>, sat_pos: Vector3<f64>) -> f64 {
    let (el, _) = calculate_el_az(user_pos, sat_pos);
    el.to_degrees()
}

/// Calculate azimuth angle in degrees
pub fn azimuth_deg(user_pos: Vector3<f64>, sat_pos: Vector3<f64>) -> f64 {
    let (_, az) = calculate_el_az(user_pos, sat_pos);
    az.to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iono_params_default() {
        let params = IonoParams::default();
        assert_eq!(params.alpha0, 0.0);
    }

    #[test]
    fn test_elevation_calculation() {
        // User at center of Earth (for simplicity)
        let user_pos = Vector3::new(0.0, 0.0, 6378137.0);
        // Satellite directly overhead
        let sat_pos = Vector3::new(0.0, 0.0, 26000000.0);

        let el = elevation_deg(user_pos, sat_pos);
        // Should be close to 90 degrees
        assert!(el > 80.0 && el < 100.0);
    }
}
