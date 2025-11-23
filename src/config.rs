use crate::constants::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub sdr: SdrConfig,
    pub acquisition: AcquisitionConfig,
    pub tracking: TrackingConfig,
    pub navigation: NavigationConfig,
    pub sbas: SbasConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdrConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Center frequency in Hz (GPS L1)
    pub center_freq: u64,
    /// Gain mode: "auto" or specific gain value
    pub gain: GainMode,
    /// Frequency correction in PPM
    pub freq_correction: i32,
    /// Enable bias-T voltage (for powering active GPS antennas)
    pub bias_tee: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GainMode {
    Auto,
    Manual(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionConfig {
    /// Coherent integration time (ms)
    pub coherent_integration_ms: usize,
    /// Maximum Doppler to search (Hz)
    pub max_doppler_hz: f64,
    /// Doppler search step (Hz)
    pub doppler_step_hz: f64,
    /// Detection threshold (sigma)
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    /// DLL bandwidth (Hz)
    pub dll_bandwidth_hz: f64,
    /// PLL bandwidth (Hz)
    pub pll_bandwidth_hz: f64,
    /// FLL bandwidth (Hz)
    pub fll_bandwidth_hz: f64,
    /// Integration time (ms)
    pub integration_ms: usize,
    /// Early-late spacing (chips)
    pub early_late_spacing: f64,
    /// Minimum CN0 for valid tracking (dB-Hz)
    pub min_cn0_db_hz: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationConfig {
    /// Minimum satellites for position fix
    pub min_satellites: usize,
    /// Position update rate (Hz)
    pub update_rate_hz: f64,
    /// Enable ionospheric corrections
    pub use_iono_correction: bool,
    /// Enable tropospheric corrections
    pub use_tropo_correction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbasConfig {
    /// Enable SBAS processing
    pub enabled: bool,
    /// SBAS PRNs to track
    pub prns: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Enable NMEA output
    pub nmea_enabled: bool,
    /// NMEA update rate (Hz)
    pub nmea_rate_hz: f64,
    /// Enable JSON output
    pub json_enabled: bool,
    /// JSON update rate (Hz)
    pub json_rate_hz: f64,
    /// Enable PPS output
    pub pps_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sdr: SdrConfig {
                sample_rate: SAMPLE_RATE_HZ,
                center_freq: GPS_L1_FREQ_HZ as u64,
                gain: GainMode::Auto,
                freq_correction: 0,
                bias_tee: false,
            },
            acquisition: AcquisitionConfig {
                coherent_integration_ms: ACQUISITION_COHERENT_MS,
                max_doppler_hz: MAX_DOPPLER_HZ,
                doppler_step_hz: DOPPLER_STEP_HZ,
                threshold: ACQUISITION_THRESHOLD,
            },
            tracking: TrackingConfig {
                dll_bandwidth_hz: DLL_BANDWIDTH_HZ,
                pll_bandwidth_hz: PLL_BANDWIDTH_HZ,
                fll_bandwidth_hz: FLL_BANDWIDTH_HZ,
                integration_ms: TRACKING_INTEGRATION_MS,
                early_late_spacing: DLL_EARLY_LATE_SPACING,
                min_cn0_db_hz: MIN_CN0_DB_HZ,
            },
            navigation: NavigationConfig {
                min_satellites: 4,
                update_rate_hz: 1.0,
                use_iono_correction: true,
                use_tropo_correction: true,
            },
            sbas: SbasConfig {
                enabled: true,
                prns: vec![131, 133, 135, 138], // WAAS satellites
            },
            output: OutputConfig {
                nmea_enabled: true,
                nmea_rate_hz: 1.0,
                json_enabled: true,
                json_rate_hz: 1.0,
                pps_enabled: true,
            },
        }
    }
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
