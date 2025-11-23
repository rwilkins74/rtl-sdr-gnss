use crate::config::Config;
use crate::constants::*;
use crate::gps::{Acquisition, Channel, ChannelState, Ephemeris};
use crate::navigation::{GnssPosition, PositionSolver, Pseudorange};
use crate::output::{JsonOutput, NmeaGenerator, PpsGenerator};
use crate::sdr::{rtlsdr_source::RtlSdrSource, SdrSource};
use crate::ui::SatelliteView;
use crate::{Error, Result};
use chrono::Utc;
use std::collections::HashMap;
use tokio::time::{interval, Duration};

pub struct GnssReceiver {
    sdr: RtlSdrSource,
    config: Config,
    channels: HashMap<u8, Channel>,
    ephemerides: HashMap<u8, Ephemeris>,
    position: Option<GnssPosition>,
    pps: PpsGenerator,
    running: bool,
}

impl GnssReceiver {
    pub fn new(device_index: u32, config: Config) -> Result<Self> {
        let sdr = RtlSdrSource::new(device_index);

        Ok(Self {
            sdr,
            config,
            channels: HashMap::new(),
            ephemerides: HashMap::new(),
            position: None,
            pps: PpsGenerator::new(),
            running: false,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting GNSS receiver...");

        // Initialize SDR
        self.sdr.init().await?;

        // Set center frequency and sample rate FIRST
        self.sdr.set_frequency(self.config.sdr.center_freq)?;
        self.sdr.set_sample_rate(self.config.sdr.sample_rate)?;

        // Set frequency correction after frequency/sample rate
        self.sdr.set_freq_correction(self.config.sdr.freq_correction)?;

        // Configure gain
        match self.config.sdr.gain {
            crate::config::GainMode::Auto => {
                self.sdr.set_agc(true)?;
            }
            crate::config::GainMode::Manual(gain) => {
                self.sdr.set_agc(false)?;
                self.sdr.set_gain(gain)?;
            }
        }

        // Enable bias-T if configured
        self.sdr.set_bias_tee(self.config.sdr.bias_tee)?;

        self.sdr.start().await?;

        if self.config.output.pps_enabled {
            self.pps.enable();
        }

        self.running = true;
        tracing::info!("GNSS receiver started successfully");

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.running = false;
        self.sdr.stop().await?;
        tracing::info!("GNSS receiver stopped");
        Ok(())
    }

    pub async fn run_headless(&mut self) -> Result<()> {
        let mut update_interval = interval(Duration::from_secs(1));

        while self.running {
            self.process_cycle().await?;

            if let Some(ref pos) = self.position {
                // Output NMEA
                if self.config.output.nmea_enabled {
                    let gga = NmeaGenerator::generate_gga(pos, Utc::now(), 1);
                    println!("{}", gga);

                    let rmc = NmeaGenerator::generate_rmc(pos, Utc::now(), 0.0, 0.0);
                    println!("{}", rmc);
                }

                // Output JSON
                if self.config.output.json_enabled {
                    let satellites: Vec<crate::output::json::SatelliteInfo> = self
                        .channels
                        .values()
                        .map(|ch| crate::output::json::SatelliteInfo {
                            prn: ch.prn,
                            elevation: 0.0,
                            azimuth: 0.0,
                            cn0: 0.0,
                            tracked: ch.state == ChannelState::Tracking,
                            used_in_fix: true,
                        })
                        .collect();

                    let json_output = JsonOutput::new(pos, satellites, "3D", true, Utc::now());
                    if let Ok(json) = json_output.to_json_compact() {
                        println!("{}", json);
                    }
                }
            }

            update_interval.tick().await;
        }

        Ok(())
    }

    async fn process_cycle(&mut self) -> Result<()> {
        // Read samples
        let samples_per_ms = self.config.sdr.sample_rate / 1000;
        let samples = self.sdr.read_samples(samples_per_ms as usize * 10).await?;

        // Acquisition phase - search for new satellites
        if self.channels.len() < 12 {
            self.acquire_satellites(&samples).await?;
        }

        // Tracking phase - track acquired satellites
        self.track_satellites(&samples).await?;

        // Navigation phase - calculate position
        self.calculate_position().await?;

        Ok(())
    }

    async fn acquire_satellites(&mut self, samples: &[num_complex::Complex<f32>]) -> Result<()> {
        let acquisition = Acquisition::new(
            self.config.acquisition.clone(),
            self.config.sdr.sample_rate as f64,
        );

        // Search for GPS satellites (PRN 1-32)
        let prns_to_search: Vec<u8> = (1..=32)
            .filter(|prn| !self.channels.contains_key(prn))
            .collect();

        if prns_to_search.is_empty() {
            return Ok(());
        }

        tracing::debug!("Searching for satellites: {:?}", prns_to_search);

        let results = acquisition.acquire_all(samples, &prns_to_search).await;

        for result in results {
            if result.acquired {
                tracing::info!("Acquired satellite PRN {}", result.prn);

                let mut channel = Channel::new(
                    result.prn,
                    self.config.tracking.clone(),
                    self.config.sdr.sample_rate as f64,
                );

                channel.initialize(result.doppler_hz, result.code_phase);
                self.channels.insert(result.prn, channel);
            }
        }

        Ok(())
    }

    async fn track_satellites(&mut self, samples: &[num_complex::Complex<f32>]) -> Result<()> {
        let mut lost_channels = Vec::new();

        for (prn, channel) in self.channels.iter_mut() {
            match channel.process(samples) {
                Ok(result) => {
                    tracing::trace!(
                        "PRN {} tracking: CN0={:.1} dB-Hz, Doppler={:.1} Hz",
                        prn,
                        result.cn0_db_hz,
                        result.carrier_freq_hz
                    );

                    // Decode navigation bits and update ephemeris
                    // This would integrate with NavMessage decoder
                }
                Err(Error::TrackingLost(prn)) => {
                    lost_channels.push(prn);
                }
                Err(e) => {
                    tracing::warn!("Error tracking PRN {}: {:?}", prn, e);
                    lost_channels.push(*prn);
                }
            }
        }

        // Remove lost channels
        for prn in lost_channels {
            self.channels.remove(&prn);
            tracing::warn!("Removed channel for PRN {}", prn);
        }

        Ok(())
    }

    async fn calculate_position(&mut self) -> Result<()> {
        // Collect pseudoranges from tracking channels
        let mut pseudoranges = Vec::new();

        for (prn, channel) in self.channels.iter() {
            if channel.state == ChannelState::Tracking {
                // In production, this would use actual tracking measurements
                let pr = Pseudorange::from_tracking(
                    *prn,
                    0.0,     // code_phase
                    GPS_CA_CHIPPING_RATE,
                    0.0,     // carrier_freq
                    0.0,     // carrier_phase
                    40.0,    // cn0
                    1000,    // lock_time_ms
                );
                pseudoranges.push(pr);
            }
        }

        if pseudoranges.len() < 4 {
            return Ok(()); // Not enough satellites for fix
        }

        // Get ephemerides
        let ephemerides: Vec<Ephemeris> = self.ephemerides.values().cloned().collect();

        if ephemerides.len() < 4 {
            return Ok(()); // Not enough ephemerides
        }

        // Solve for position
        let solver = PositionSolver::default();
        let gps_time = 0.0; // This would come from navigation message

        match solver.solve(&pseudoranges, &ephemerides, gps_time) {
            Ok(solution) => {
                self.position = Some(solution.position.clone());

                tracing::info!(
                    "Position: {:.7}, {:.7}, {:.2}m (sats: {}, HDOP: {:.2})",
                    solution.position.latitude,
                    solution.position.longitude,
                    solution.position.altitude,
                    solution.position.num_satellites,
                    solution.position.hdop
                );

                // Generate PPS
                if self.config.output.pps_enabled {
                    self.pps.generate_pulse(gps_time).await;
                }
            }
            Err(e) => {
                tracing::debug!("Position solver failed: {:?}", e);
            }
        }

        Ok(())
    }

    pub fn get_position(&self) -> Option<GnssPosition> {
        self.position.clone()
    }

    pub fn get_satellite_views(&self) -> Vec<SatelliteView> {
        self.channels
            .values()
            .map(|ch| SatelliteView {
                prn: ch.prn,
                elevation: 0.0,
                azimuth: 0.0,
                cn0: 0.0,
                state: format!("{:?}", ch.state),
                used_in_fix: ch.state == ChannelState::Tracking,
            })
            .collect()
    }

    pub fn get_status(&self) -> String {
        format!(
            "Tracking {} satellites | {} channels active",
            self.channels
                .values()
                .filter(|ch| ch.state == ChannelState::Tracking)
                .count(),
            self.channels.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_creation() {
        let config = Config::default();
        let receiver = GnssReceiver::new(0, config);
        assert!(receiver.is_ok());
    }
}
