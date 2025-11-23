use crate::sdr::SdrSource;
use crate::{Error, Result};
use num_complex::Complex;
use std::sync::{Arc, Mutex};

/// Wrapper around RTLSDRDevice that implements Send
/// Safety: RTL-SDR operations are synchronized and device handle is not shared between threads
struct SendableDevice(rtlsdr::RTLSDRDevice);

unsafe impl Send for SendableDevice {}

pub struct RtlSdrSource {
    device_index: u32,
    center_freq: u64,
    sample_rate: u32,
    gain: Option<f32>,
    device: Option<SendableDevice>,
    running: Arc<Mutex<bool>>,
    bias_tee_enabled: bool,
}

impl RtlSdrSource {
    pub fn new(device_index: u32) -> Self {
        Self {
            device_index,
            center_freq: 0,
            sample_rate: 0,
            gain: None,
            device: None,
            running: Arc::new(Mutex::new(false)),
            bias_tee_enabled: false,
        }
    }

    fn convert_iq_to_complex(buffer: &[u8]) -> Vec<Complex<f32>> {
        buffer
            .chunks_exact(2)
            .map(|iq| {
                // RTL-SDR gives unsigned 8-bit IQ, convert to signed float
                let i = (iq[0] as f32 - 127.5) / 127.5;
                let q = (iq[1] as f32 - 127.5) / 127.5;
                Complex::new(i, q)
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl SdrSource for RtlSdrSource {
    async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing RTL-SDR device {}", self.device_index);

        // Open RTL-SDR device
        let device = rtlsdr::open(self.device_index as i32)
            .map_err(|e| Error::RtlSdr(format!("Failed to open device {}: {:?}", self.device_index, e)))?;

        tracing::info!("RTL-SDR device opened successfully");
        tracing::info!("Device index: {}", self.device_index);

        self.device = Some(SendableDevice(device));

        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        if let Some(ref mut device) = self.device {
            device.0
                .reset_buffer()
                .map_err(|e| Error::RtlSdr(format!("Failed to reset buffer: {:?}", e)))?;

            *self.running.lock().unwrap() = true;
            tracing::info!("RTL-SDR started");
            Ok(())
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.lock().unwrap() = false;
        tracing::info!("RTL-SDR stopped");
        Ok(())
    }

    async fn read_samples(&mut self, count: usize) -> Result<Vec<Complex<f32>>> {
        if let Some(ref mut device) = self.device {
            let byte_count = count * 2; // 2 bytes per complex sample (I + Q)

            // read_sync in rtlsdr v0.1 takes a length and returns a Vec<u8>
            let buffer = device.0
                .read_sync(byte_count)
                .map_err(|e| Error::RtlSdr(format!("Failed to read samples: {:?}", e)))?;

            Ok(Self::convert_iq_to_complex(&buffer))
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    fn set_frequency(&mut self, freq: u64) -> Result<()> {
        if let Some(ref mut device) = self.device {
            device.0
                .set_center_freq(freq as u32)
                .map_err(|e| Error::RtlSdr(format!("Failed to set frequency: {:?}", e)))?;

            let actual_freq = device.0.get_center_freq()
                .map_err(|e| Error::RtlSdr(format!("Failed to get center frequency: {:?}", e)))?;
            self.center_freq = actual_freq as u64;
            tracing::info!("Center frequency set to {} Hz", self.center_freq);
            Ok(())
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    fn set_sample_rate(&mut self, rate: u32) -> Result<()> {
        if let Some(ref mut device) = self.device {
            device.0
                .set_sample_rate(rate)
                .map_err(|e| Error::RtlSdr(format!("Failed to set sample rate: {:?}", e)))?;

            let actual_rate = device.0.get_sample_rate()
                .map_err(|e| Error::RtlSdr(format!("Failed to get sample rate: {:?}", e)))?;
            self.sample_rate = actual_rate;
            tracing::info!("Sample rate set to {} Hz", self.sample_rate);
            Ok(())
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    fn set_gain(&mut self, gain: f32) -> Result<()> {
        if let Some(ref mut device) = self.device {
            let gain_tenths = (gain * 10.0) as i32;
            device.0
                .set_tuner_gain(gain_tenths)
                .map_err(|e| Error::RtlSdr(format!("Failed to set gain: {:?}", e)))?;

            self.gain = Some(gain);
            tracing::info!("Gain set to {} dB", gain);
            Ok(())
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    fn set_agc(&mut self, enable: bool) -> Result<()> {
        if let Some(ref mut device) = self.device {
            if enable {
                // Set AGC mode - rtlsdr v0.1 uses set_tuner_gain_mode
                device.0
                    .set_tuner_gain_mode(false) // false = automatic gain
                    .map_err(|e| Error::RtlSdr(format!("Failed to enable AGC: {:?}", e)))?;
                tracing::info!("AGC enabled (automatic gain mode)");
            } else {
                // Manual gain mode
                device.0
                    .set_tuner_gain_mode(true) // true = manual gain
                    .map_err(|e| Error::RtlSdr(format!("Failed to set manual gain: {:?}", e)))?;
                tracing::info!("Manual gain mode enabled");
            }
            Ok(())
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    fn set_bias_tee(&mut self, enable: bool) -> Result<()> {
        if self.device.is_some() {
            // Note: The rtlsdr crate v0.1 doesn't expose bias-T control
            // For real bias-T support, you need to either:
            // 1. Use rtl_biast command-line tool before starting the receiver
            // 2. Upgrade to rtlsdr-rs fork with bias-T support
            // 3. Use direct librtlsdr FFI calls
            //
            // For RTL-SDR Blog V3 and similar dongles, bias-T is controlled via GPIO
            // and requires calling rtlsdr_set_bias_tee() from librtlsdr

            self.bias_tee_enabled = enable;

            if enable {
                tracing::warn!("Bias-T requested but not supported by rtlsdr crate v0.1");
                tracing::warn!("To enable bias-T:");
                tracing::warn!("  1. Run 'rtl_biast -b 1' before starting this program");
                tracing::warn!("  2. Or upgrade to a newer rtlsdr crate with bias-T support");
                tracing::warn!("WARNING: Only enable bias-T if your antenna requires power!");
            } else {
                tracing::info!("Bias-T disable requested (not implemented in rtlsdr v0.1)");
                tracing::info!("Run 'rtl_biast -b 0' if you need to disable bias-T");
            }

            Ok(())
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    fn set_freq_correction(&mut self, ppm: i32) -> Result<()> {
        // Skip if no correction needed
        if ppm == 0 {
            tracing::info!("Frequency correction set to 0 PPM (no correction)");
            return Ok(());
        }

        if let Some(ref mut device) = self.device {
            match device.0.set_freq_correction(ppm) {
                Ok(()) => {
                    tracing::info!("Frequency correction set to {} PPM", ppm);
                    Ok(())
                }
                Err(e) => {
                    // Some devices/tuners don't support frequency correction
                    // Log a warning but don't fail
                    tracing::warn!(
                        "Failed to set frequency correction to {} PPM: {:?}",
                        ppm, e
                    );
                    tracing::warn!(
                        "This may be normal for some RTL-SDR devices/tuners. Continuing without correction."
                    );
                    Ok(())
                }
            }
        } else {
            Err(Error::RtlSdr("Device not initialized".to_string()))
        }
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn get_frequency(&self) -> u64 {
        self.center_freq
    }
}

impl Drop for RtlSdrSource {
    fn drop(&mut self) {
        // Disable bias-T on shutdown for safety
        if self.bias_tee_enabled {
            tracing::info!("Shutting down - disabling bias-T");
            let _ = self.set_bias_tee(false);
        }
    }
}
