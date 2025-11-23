use crate::navigation::GnssPosition;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteInfo {
    pub prn: u8,
    pub elevation: f64,
    pub azimuth: f64,
    pub cn0: f64,
    pub tracked: bool,
    pub used_in_fix: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonOutput {
    pub timestamp: String,
    pub position: PositionOutput,
    pub satellites: Vec<SatelliteInfo>,
    pub fix_quality: FixQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionOutput {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub hdop: f64,
    pub vdop: f64,
    pub pdop: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixQuality {
    pub fix_type: String,
    pub num_satellites: usize,
    pub converged: bool,
}

impl JsonOutput {
    pub fn new(
        position: &GnssPosition,
        satellites: Vec<SatelliteInfo>,
        fix_type: &str,
        converged: bool,
        time: DateTime<Utc>,
    ) -> Self {
        Self {
            timestamp: time.to_rfc3339(),
            position: PositionOutput {
                latitude: position.latitude,
                longitude: position.longitude,
                altitude: position.altitude,
                hdop: position.hdop,
                vdop: position.vdop,
                pdop: position.pdop,
            },
            satellites,
            fix_quality: FixQuality {
                fix_type: fix_type.to_string(),
                num_satellites: position.num_satellites,
                converged,
            },
        }
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_json_compact(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Vector3;

    #[test]
    fn test_json_output() {
        let position = GnssPosition::from_ecef(Vector3::new(0.0, 0.0, 6378137.0), 0.0, 4);
        let satellites = vec![
            SatelliteInfo {
                prn: 1,
                elevation: 45.0,
                azimuth: 180.0,
                cn0: 42.0,
                tracked: true,
                used_in_fix: true,
            },
        ];

        let output = JsonOutput::new(&position, satellites, "3D", true, Utc::now());
        let json = output.to_json().unwrap();

        assert!(json.contains("latitude"));
        assert!(json.contains("longitude"));
        assert!(json.contains("satellites"));
    }

    #[test]
    fn test_json_compact() {
        let position = GnssPosition::from_ecef(Vector3::new(0.0, 0.0, 6378137.0), 0.0, 4);
        let satellites = vec![];

        let output = JsonOutput::new(&position, satellites, "3D", true, Utc::now());
        let json = output.to_json_compact().unwrap();

        assert!(!json.contains('\n')); // Compact format has no newlines
    }
}
