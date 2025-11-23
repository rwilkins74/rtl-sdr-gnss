use crate::navigation::GnssPosition;
use chrono::{DateTime, Datelike, Timelike, Utc};

pub struct NmeaGenerator;

impl NmeaGenerator {
    /// Generate NMEA checksum
    fn checksum(sentence: &str) -> String {
        let mut checksum: u8 = 0;
        for byte in sentence.bytes() {
            checksum ^= byte;
        }
        format!("{:02X}", checksum)
    }

    /// Generate GGA sentence (Global Positioning System Fix Data)
    pub fn generate_gga(
        position: &GnssPosition,
        time: DateTime<Utc>,
        fix_quality: u8,
    ) -> String {
        let lat_deg = position.latitude.abs();
        let lat_min = (lat_deg - lat_deg.floor()) * 60.0;
        let lat_dir = if position.latitude >= 0.0 { 'N' } else { 'S' };

        let lon_deg = position.longitude.abs();
        let lon_min = (lon_deg - lon_deg.floor()) * 60.0;
        let lon_dir = if position.longitude >= 0.0 { 'E' } else { 'W' };

        let sentence = format!(
            "GPGGA,{:02}{:02}{:06.3},{:02}{:08.5},{},{:03}{:08.5},{},{},{},{:.1},{:.1},M,0.0,M,,",
            time.hour(),
            time.minute(),
            time.second() as f64 + (time.nanosecond() as f64 / 1e9),
            lat_deg.floor() as u32,
            lat_min,
            lat_dir,
            lon_deg.floor() as u32,
            lon_min,
            lon_dir,
            fix_quality,
            position.num_satellites,
            position.hdop,
            position.altitude
        );

        format!("${}*{}", sentence, Self::checksum(&sentence))
    }

    /// Generate RMC sentence (Recommended Minimum Navigation Information)
    pub fn generate_rmc(
        position: &GnssPosition,
        time: DateTime<Utc>,
        speed_knots: f64,
        track_deg: f64,
    ) -> String {
        let lat_deg = position.latitude.abs();
        let lat_min = (lat_deg - lat_deg.floor()) * 60.0;
        let lat_dir = if position.latitude >= 0.0 { 'N' } else { 'S' };

        let lon_deg = position.longitude.abs();
        let lon_min = (lon_deg - lon_deg.floor()) * 60.0;
        let lon_dir = if position.longitude >= 0.0 { 'E' } else { 'W' };

        let sentence = format!(
            "GPRMC,{:02}{:02}{:06.3},A,{:02}{:08.5},{},{:03}{:08.5},{},{:.1},{:.1},{:02}{:02}{:02},,,A",
            time.hour(),
            time.minute(),
            time.second() as f64 + (time.nanosecond() as f64 / 1e9),
            lat_deg.floor() as u32,
            lat_min,
            lat_dir,
            lon_deg.floor() as u32,
            lon_min,
            lon_dir,
            speed_knots,
            track_deg,
            time.day(),
            time.month(),
            time.year() % 100
        );

        format!("${}*{}", sentence, Self::checksum(&sentence))
    }

    /// Generate GSA sentence (GPS DOP and Active Satellites)
    pub fn generate_gsa(
        position: &GnssPosition,
        satellite_prns: &[u8],
        fix_type: u8,
    ) -> String {
        let mut prn_fields = String::new();
        for i in 0..12 {
            if i < satellite_prns.len() {
                prn_fields.push_str(&format!("{:02},", satellite_prns[i]));
            } else {
                prn_fields.push_str(",");
            }
        }

        let sentence = format!(
            "GPGSA,A,{},{}{:.2},{:.2},{:.2}",
            fix_type,
            prn_fields,
            position.pdop,
            position.hdop,
            position.vdop
        );

        format!("${}*{}", sentence, Self::checksum(&sentence))
    }

    /// Generate GSV sentence (GPS Satellites in View)
    pub fn generate_gsv(
        satellites: &[(u8, f64, f64, f64)], // (PRN, elevation, azimuth, SNR)
    ) -> Vec<String> {
        let mut sentences = Vec::new();
        let num_messages = (satellites.len() + 3) / 4;

        for msg_num in 0..num_messages {
            let start = msg_num * 4;
            let end = (start + 4).min(satellites.len());

            let mut sentence = format!(
                "GPGSV,{},{},{}",
                num_messages,
                msg_num + 1,
                satellites.len()
            );

            for &(prn, elev, azim, snr) in &satellites[start..end] {
                sentence.push_str(&format!(",{:02},{:.0},{:.0},{:.0}", prn, elev, azim, snr));
            }

            sentences.push(format!("${}*{}", sentence, Self::checksum(&sentence)));
        }

        sentences
    }

    /// Generate VTG sentence (Track Made Good and Ground Speed)
    pub fn generate_vtg(track_deg: f64, speed_knots: f64) -> String {
        let speed_kmh = speed_knots * 1.852;

        let sentence = format!(
            "GPVTG,{:.1},T,,M,{:.1},N,{:.1},K,A",
            track_deg, speed_knots, speed_kmh
        );

        format!("${}*{}", sentence, Self::checksum(&sentence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::GnssPosition;
    use nalgebra::Vector3;

    #[test]
    fn test_checksum() {
        let checksum = NmeaGenerator::checksum("GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert_eq!(checksum.len(), 2);
    }

    #[test]
    fn test_gga_generation() {
        let position = GnssPosition::from_ecef(Vector3::new(0.0, 0.0, 6378137.0), 0.0, 8);
        let time = Utc::now();
        let gga = NmeaGenerator::generate_gga(&position, time, 1);

        assert!(gga.starts_with("$GPGGA"));
        assert!(gga.contains('*'));
    }

    #[test]
    fn test_rmc_generation() {
        let position = GnssPosition::from_ecef(Vector3::new(0.0, 0.0, 6378137.0), 0.0, 8);
        let time = Utc::now();
        let rmc = NmeaGenerator::generate_rmc(&position, time, 0.0, 0.0);

        assert!(rmc.starts_with("$GPRMC"));
        assert!(rmc.contains('*'));
    }

    #[test]
    fn test_gsv_generation() {
        let satellites = vec![
            (1, 45.0, 180.0, 42.0),
            (2, 30.0, 90.0, 38.0),
            (3, 60.0, 270.0, 45.0),
        ];

        let gsv_sentences = NmeaGenerator::generate_gsv(&satellites);
        assert!(!gsv_sentences.is_empty());
        assert!(gsv_sentences[0].starts_with("$GPGSV"));
    }
}
