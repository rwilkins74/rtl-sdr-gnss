pub mod rtlsdr_source;

use crate::Result;
use num_complex::Complex;

/// Trait for SDR sources
#[async_trait::async_trait]
pub trait SdrSource: Send {
    /// Initialize the SDR device
    async fn init(&mut self) -> Result<()>;

    /// Start receiving samples
    async fn start(&mut self) -> Result<()>;

    /// Stop receiving samples
    async fn stop(&mut self) -> Result<()>;

    /// Read IQ samples
    async fn read_samples(&mut self, count: usize) -> Result<Vec<Complex<f32>>>;

    /// Set center frequency
    fn set_frequency(&mut self, freq: u64) -> Result<()>;

    /// Set sample rate
    fn set_sample_rate(&mut self, rate: u32) -> Result<()>;

    /// Set gain
    fn set_gain(&mut self, gain: f32) -> Result<()>;

    /// Enable automatic gain control
    fn set_agc(&mut self, enable: bool) -> Result<()>;

    /// Enable/disable bias-T voltage (for powering LNA in GPS antennas)
    fn set_bias_tee(&mut self, enable: bool) -> Result<()>;

    /// Set frequency correction in PPM
    fn set_freq_correction(&mut self, ppm: i32) -> Result<()>;

    /// Get actual sample rate
    fn get_sample_rate(&self) -> u32;

    /// Get actual center frequency
    fn get_frequency(&self) -> u64;
}
