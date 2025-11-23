use super::SbasMessage;
use nalgebra::Vector3;

/// Apply SBAS fast corrections to pseudorange
pub fn apply_fast_correction(
    pseudorange: f64,
    fast_corr: f64,
    udrei: u8, // User Differential Range Error Indicator
) -> f64 {
    // Fast corrections are updated frequently (< 6 seconds)
    // Apply only if UDREI indicates correction is valid
    if udrei < 14 {
        pseudorange - fast_corr
    } else {
        pseudorange
    }
}

/// Apply SBAS long-term corrections
pub fn apply_long_term_correction(
    satellite_position: Vector3<f64>,
    velocity: Vector3<f64>,
    delta_x: f64,
    delta_y: f64,
    delta_z: f64,
    delta_clock: f64,
) -> (Vector3<f64>, f64) {
    // Apply position corrections
    let corrected_pos = Vector3::new(
        satellite_position.x + delta_x,
        satellite_position.y + delta_y,
        satellite_position.z + delta_z,
    );

    (corrected_pos, delta_clock)
}

/// Get SBAS UDRE (User Differential Range Error) from UDREI
pub fn udrei_to_udre(udrei: u8) -> f64 {
    // Convert UDREI to UDRE in meters (Table A-37 from RTCA DO-229D)
    match udrei {
        0 => 0.75,
        1 => 1.0,
        2 => 1.25,
        3 => 1.75,
        4 => 2.25,
        5 => 3.0,
        6 => 3.75,
        7 => 4.75,
        8 => 5.75,
        9 => 7.0,
        10 => 8.75,
        11 => 11.25,
        12 => 14.75,
        13 => 20.0,
        14 => 31.0,
        15 => f64::INFINITY, // Do not use
        _ => f64::INFINITY,
    }
}

/// Apply SBAS ionospheric grid corrections
pub fn apply_iono_grid_correction(
    user_lat: f64,
    user_lon: f64,
    pierce_point_lat: f64,
    pierce_point_lon: f64,
    grid_corrections: &[(f64, f64, f64)], // (lat, lon, delay)
) -> f64 {
    // Simplified bilinear interpolation of ionospheric grid
    // In production, implement full SBAS ionospheric model

    if grid_corrections.is_empty() {
        return 0.0;
    }

    // Find nearest grid point
    let mut min_dist = f64::INFINITY;
    let mut nearest_delay = 0.0;

    for &(lat, lon, delay) in grid_corrections {
        let dlat = pierce_point_lat - lat;
        let dlon = pierce_point_lon - lon;
        let dist = (dlat * dlat + dlon * dlon).sqrt();

        if dist < min_dist {
            min_dist = dist;
            nearest_delay = delay;
        }
    }

    nearest_delay
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udrei_conversion() {
        assert_eq!(udrei_to_udre(0), 0.75);
        assert_eq!(udrei_to_udre(15), f64::INFINITY);
    }

    #[test]
    fn test_fast_correction() {
        let pr = 20_000_000.0; // 20,000 km
        let corrected = apply_fast_correction(pr, 10.0, 3);
        assert_eq!(corrected, pr - 10.0);

        // Invalid UDREI should not apply correction
        let uncorrected = apply_fast_correction(pr, 10.0, 15);
        assert_eq!(uncorrected, pr);
    }
}
