use crate::config::TrackingConfig;
use crate::constants::*;
use crate::gps::ca_code::CaCodeGenerator;
use crate::signal::{correlator::Correlator, discriminator, loop_filter::LoopFilter};
use crate::{Error, Result};
use num_complex::Complex;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Idle,
    PullIn,     // Initial acquisition to tracking transition
    Tracking,   // Normal tracking
    Lost,       // Lost lock
}

#[derive(Debug, Clone)]
pub struct TrackingResult {
    pub prn: u8,
    pub carrier_freq_hz: f64,
    pub code_freq_hz: f64,
    pub code_phase: f64,
    pub carrier_phase: f64,
    pub prompt: Complex<f32>,
    pub cn0_db_hz: f64,
    pub lock_time_ms: u64,
    pub nav_bit: Option<i8>,
}

pub struct Channel {
    pub prn: u8,
    pub state: ChannelState,

    // Configuration
    config: TrackingConfig,
    sample_rate: f64,

    // Code tracking
    code_nco: f64,          // Code NCO (chips)
    code_freq: f64,         // Code frequency (Hz)
    dll_filter: LoopFilter,

    // Carrier tracking
    carrier_nco: f64,       // Carrier NCO (radians)
    carrier_freq: f64,      // Carrier Doppler (Hz)
    pll_filter: LoopFilter,
    fll_filter: LoopFilter,

    // C/A code
    ca_code: Vec<f32>,
    samples_per_chip: f64,

    // Correlator
    correlator: Correlator,

    // Measurements
    prompt_history: VecDeque<Complex<f32>>,
    cn0_db_hz: f64,
    lock_time_ms: u64,
    integration_count: u64,

    // Navigation data
    nav_bit_sync: bool,
    nav_bit_history: Vec<i8>,
    last_prompt_sign: i8,
    bit_integrations: u32,
}

impl Channel {
    pub fn new(prn: u8, config: TrackingConfig, sample_rate: f64) -> Self {
        let samples_per_chip = sample_rate / GPS_CA_CHIPPING_RATE;
        let ca_code = CaCodeGenerator::generate_sampled(prn, sample_rate, GPS_CA_CHIPPING_RATE);

        let integration_time_s = config.integration_ms as f64 / 1000.0;
        let early_late_spacing = config.early_late_spacing;
        let dll_bw = config.dll_bandwidth_hz;
        let pll_bw = config.pll_bandwidth_hz;
        let fll_bw = config.fll_bandwidth_hz;

        Self {
            prn,
            state: ChannelState::Idle,
            config,
            sample_rate,
            code_nco: 0.0,
            code_freq: GPS_CA_CHIPPING_RATE,
            dll_filter: LoopFilter::new(dll_bw, 0.707, integration_time_s),
            carrier_nco: 0.0,
            carrier_freq: 0.0,
            pll_filter: LoopFilter::new(pll_bw, 0.707, integration_time_s),
            fll_filter: LoopFilter::new(fll_bw, 0.707, integration_time_s),
            ca_code,
            samples_per_chip,
            correlator: Correlator::new(early_late_spacing),
            prompt_history: VecDeque::with_capacity(1000),
            cn0_db_hz: 0.0,
            lock_time_ms: 0,
            integration_count: 0,
            nav_bit_sync: false,
            nav_bit_history: Vec::new(),
            last_prompt_sign: 0,
            bit_integrations: 0,
        }
    }

    /// Initialize channel with acquisition results
    pub fn initialize(&mut self, doppler_hz: f64, code_phase: usize) {
        self.carrier_freq = doppler_hz;
        self.code_nco = code_phase as f64;
        self.state = ChannelState::PullIn;
        self.lock_time_ms = 0;
        self.integration_count = 0;

        tracing::info!(
            "Channel initialized for PRN {}: Doppler={:.1} Hz, Code Phase={}",
            self.prn,
            doppler_hz,
            code_phase
        );
    }

    /// Process one integration period
    pub fn process(&mut self, signal: &[Complex<f32>]) -> Result<TrackingResult> {
        if self.state == ChannelState::Idle || self.state == ChannelState::Lost {
            return Err(Error::TrackingLost(self.prn));
        }

        // Apply carrier wipeoff
        let signal_baseband = self.carrier_wipeoff(signal);

        // Correlate with E/P/L codes
        let (early, prompt, late) = self.correlator.correlate_epl(
            &signal_baseband,
            &self.ca_code,
            self.code_nco / self.samples_per_chip,
            self.samples_per_chip,
        );

        // DLL discriminator and filter
        let dll_error = discriminator::dll_nemlp(early, late);
        let code_correction = self.dll_filter.update(dll_error);
        self.code_freq = GPS_CA_CHIPPING_RATE + code_correction;

        // PLL discriminator and filter
        let pll_error = discriminator::pll_costas(prompt);
        let carrier_correction = self.pll_filter.update(pll_error);
        self.carrier_freq += carrier_correction;

        // Update NCOs
        let samples_per_integration = signal.len();
        let integration_time_s = samples_per_integration as f64 / self.sample_rate;

        self.code_nco += self.code_freq * integration_time_s;
        self.code_nco %= GPS_CA_CODE_LENGTH as f64;

        self.carrier_nco += 2.0 * std::f64::consts::PI * self.carrier_freq * integration_time_s;
        self.carrier_nco %= 2.0 * std::f64::consts::PI;

        // Store prompt for CN0 estimation
        self.prompt_history.push_back(prompt);
        if self.prompt_history.len() > 1000 {
            self.prompt_history.pop_front();
        }

        // Estimate CN0
        self.estimate_cn0(integration_time_s);

        // Check lock status
        if self.cn0_db_hz < self.config.min_cn0_db_hz {
            tracing::warn!("PRN {} lost lock: CN0={:.1} dB-Hz", self.prn, self.cn0_db_hz);
            self.state = ChannelState::Lost;
            return Err(Error::TrackingLost(self.prn));
        }

        // Transition from pull-in to tracking
        if self.state == ChannelState::PullIn && self.lock_time_ms > 100 {
            self.state = ChannelState::Tracking;
            tracing::info!("PRN {} locked and tracking", self.prn);
        }

        self.lock_time_ms += self.config.integration_ms as u64;
        self.integration_count += 1;

        // Decode navigation bit (every 20ms)
        let nav_bit = self.decode_nav_bit(prompt);

        Ok(TrackingResult {
            prn: self.prn,
            carrier_freq_hz: self.carrier_freq,
            code_freq_hz: self.code_freq,
            code_phase: self.code_nco,
            carrier_phase: self.carrier_nco,
            prompt,
            cn0_db_hz: self.cn0_db_hz,
            lock_time_ms: self.lock_time_ms,
            nav_bit,
        })
    }

    fn carrier_wipeoff(&self, signal: &[Complex<f32>]) -> Vec<Complex<f32>> {
        signal
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let phase = self.carrier_nco
                    + 2.0 * std::f64::consts::PI * self.carrier_freq * (i as f64) / self.sample_rate;
                let carrier = Complex::new(phase.cos() as f32, -phase.sin() as f32);
                s * carrier
            })
            .collect()
    }

    fn estimate_cn0(&mut self, integration_time_s: f64) {
        if self.prompt_history.len() < 10 {
            return;
        }

        let i_values: Vec<f32> = self.prompt_history.iter().map(|p| p.re).collect();
        let q_values: Vec<f32> = self.prompt_history.iter().map(|p| p.im).collect();

        self.cn0_db_hz = discriminator::cn0_beaulieu(&i_values, &q_values, integration_time_s);
    }

    fn decode_nav_bit(&mut self, prompt: Complex<f32>) -> Option<i8> {
        // Sign of prompt indicates data bit (accounting for 180° phase ambiguity)
        let current_sign = if prompt.re > 0.0 { 1 } else { -1 };

        // Count integrations per bit (20ms = 20 * 1ms integrations typically)
        self.bit_integrations += 1;

        if self.bit_integrations >= (NAV_BIT_DURATION_MS / self.config.integration_ms as f64) as u32 {
            self.bit_integrations = 0;

            // Detect bit transition for bit sync
            if !self.nav_bit_sync {
                if self.last_prompt_sign != 0 && self.last_prompt_sign != current_sign {
                    self.nav_bit_sync = true;
                    tracing::debug!("PRN {} achieved bit sync", self.prn);
                }
            }

            self.last_prompt_sign = current_sign;

            if self.nav_bit_sync {
                return Some(current_sign);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let config = TrackingConfig {
            dll_bandwidth_hz: 2.0,
            pll_bandwidth_hz: 25.0,
            fll_bandwidth_hz: 5.0,
            integration_ms: 1,
            early_late_spacing: 0.5,
            min_cn0_db_hz: 25.0,
        };

        let channel = Channel::new(1, config, 2_048_000.0);
        assert_eq!(channel.prn, 1);
        assert_eq!(channel.state, ChannelState::Idle);
    }

    #[test]
    fn test_channel_initialization() {
        let config = TrackingConfig {
            dll_bandwidth_hz: 2.0,
            pll_bandwidth_hz: 25.0,
            fll_bandwidth_hz: 5.0,
            integration_ms: 1,
            early_late_spacing: 0.5,
            min_cn0_db_hz: 25.0,
        };

        let mut channel = Channel::new(1, config, 2_048_000.0);
        channel.initialize(1500.0, 100);

        assert_eq!(channel.state, ChannelState::PullIn);
        assert_eq!(channel.carrier_freq, 1500.0);
    }
}
