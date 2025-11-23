/// GPS L1 carrier frequency (Hz)
pub const GPS_L1_FREQ_HZ: f64 = 1_575_420_000.0;

/// GPS C/A code chipping rate (chips/sec)
pub const GPS_CA_CHIPPING_RATE: f64 = 1_023_000.0;

/// GPS C/A code length (chips)
pub const GPS_CA_CODE_LENGTH: usize = 1023;

/// Number of GPS satellites (PRN 1-32)
pub const NUM_GPS_SATELLITES: usize = 32;

/// SBAS PRN range (120-158)
pub const SBAS_PRN_MIN: u8 = 120;
pub const SBAS_PRN_MAX: u8 = 158;

/// Sample rate for RTL-SDR (Hz)
pub const SAMPLE_RATE_HZ: u32 = 2_048_000;

/// Intermediate frequency (Hz) - we tune slightly off L1
pub const IF_FREQ_HZ: f64 = 0.0; // Direct sampling

/// Speed of light (m/s)
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// Earth's gravitational constant (m^3/s^2)
pub const EARTH_GM: f64 = 3.986005e14;

/// Earth's rotation rate (rad/s)
pub const EARTH_OMEGA_DOT: f64 = 7.2921151467e-5;

/// WGS84 Earth semi-major axis (m)
pub const EARTH_A: f64 = 6_378_137.0;

/// WGS84 Earth flattening
pub const EARTH_F: f64 = 1.0 / 298.257223563;

/// WGS84 Earth eccentricity squared
pub const EARTH_E2: f64 = EARTH_F * (2.0 - EARTH_F);

/// GPS epoch (1980-01-06 00:00:00 UTC)
pub const GPS_EPOCH_UNIX: i64 = 315_964_800;

/// GPS week rollover (weeks since 1980-01-06)
pub const GPS_WEEK_ROLLOVER: u32 = 2048;

/// Navigation message bit duration (ms)
pub const NAV_BIT_DURATION_MS: f64 = 20.0;

/// Subframe duration (ms)
pub const SUBFRAME_DURATION_MS: f64 = 6000.0;

/// Bits per subframe
pub const BITS_PER_SUBFRAME: usize = 300;

/// Correlation threshold for acquisition
pub const ACQUISITION_THRESHOLD: f64 = 2.5;

/// Number of coherent integrations for acquisition
pub const ACQUISITION_COHERENT_MS: usize = 1;

/// DLL (Delay Lock Loop) bandwidth (Hz)
pub const DLL_BANDWIDTH_HZ: f64 = 2.0;

/// PLL (Phase Lock Loop) bandwidth (Hz)
pub const PLL_BANDWIDTH_HZ: f64 = 25.0;

/// FLL (Frequency Lock Loop) bandwidth (Hz)
pub const FLL_BANDWIDTH_HZ: f64 = 5.0;

/// Early-late spacing for DLL (chips)
pub const DLL_EARLY_LATE_SPACING: f64 = 0.5;

/// Integration time for tracking (ms)
pub const TRACKING_INTEGRATION_MS: usize = 1;

/// CN0 estimation window (ms)
pub const CN0_WINDOW_MS: usize = 1000;

/// Minimum CN0 for tracking (dB-Hz)
pub const MIN_CN0_DB_HZ: f64 = 25.0;

/// Maximum Doppler search range (Hz)
pub const MAX_DOPPLER_HZ: f64 = 10_000.0;

/// Doppler search step (Hz)
pub const DOPPLER_STEP_HZ: f64 = 500.0;
