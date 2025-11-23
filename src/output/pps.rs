use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;

/// PPS (Pulse Per Second) Generator
pub struct PpsGenerator {
    enabled: Arc<AtomicBool>,
    last_pps: Option<Instant>,
}

impl PpsGenerator {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            last_pps: None,
        }
    }

    pub fn enable(&mut self) {
        self.enabled.store(true, Ordering::Relaxed);
        tracing::info!("PPS output enabled");
    }

    pub fn disable(&mut self) {
        self.enabled.store(false, Ordering::Relaxed);
        tracing::info!("PPS output disabled");
    }

    /// Generate PPS pulse aligned to GPS second
    pub async fn generate_pulse(&mut self, gps_time_s: f64) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        // Calculate time until next second boundary
        let fractional_second = gps_time_s - gps_time_s.floor();
        let time_to_next_second = 1.0 - fractional_second;

        // Only generate pulse if we're close to a second boundary (within 10ms)
        if time_to_next_second < 0.01 || time_to_next_second > 0.99 {
            let now = Instant::now();

            // Check if enough time has passed since last pulse (debounce)
            if let Some(last) = self.last_pps {
                if now.duration_since(last) < Duration::from_millis(900) {
                    return false;
                }
            }

            self.last_pps = Some(now);
            self.send_pulse().await;
            return true;
        }

        false
    }

    /// Send PPS pulse (platform-specific implementation)
    async fn send_pulse(&self) {
        // This is a placeholder implementation
        // In a real system, this would:
        // - Toggle a GPIO pin on embedded systems
        // - Send a network message (NTP-like)
        // - Write to a serial port
        // - Update a shared memory location
        //
        // For now, we'll just log it
        tracing::debug!("PPS pulse generated at {:?}", Instant::now());

        // In production, you might do something like:
        // #[cfg(target_os = "linux")]
        // {
        //     // Use GPIO sysfs or character device
        //     // e.g., write to /sys/class/gpio/gpioXX/value
        // }
        //
        // #[cfg(feature = "network_pps")]
        // {
        //     // Send UDP packet to PPS server
        // }
    }

    /// Calculate GPS time from system time (rough approximation)
    pub fn system_time_to_gps(system_time: Duration) -> f64 {
        // GPS epoch: January 6, 1980 00:00:00 UTC
        // This is a simplified conversion
        // In production, you'd need to account for leap seconds
        let gps_epoch_unix = 315_964_800u64; // GPS epoch in Unix time
        let unix_time = system_time.as_secs();

        if unix_time > gps_epoch_unix {
            (unix_time - gps_epoch_unix) as f64
        } else {
            0.0
        }
    }
}

impl Default for PpsGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pps_creation() {
        let pps = PpsGenerator::new();
        assert!(!pps.enabled.load(Ordering::Relaxed));
    }

    #[test]
    fn test_pps_enable_disable() {
        let mut pps = PpsGenerator::new();
        pps.enable();
        assert!(pps.enabled.load(Ordering::Relaxed));

        pps.disable();
        assert!(!pps.enabled.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_pulse_generation() {
        let mut pps = PpsGenerator::new();
        pps.enable();

        // Test pulse at second boundary
        let result = pps.generate_pulse(123456.999).await;
        assert!(result);
    }

    #[test]
    fn test_gps_time_conversion() {
        let system_time = Duration::from_secs(1_700_000_000);
        let gps_time = PpsGenerator::system_time_to_gps(system_time);
        assert!(gps_time > 0.0);
    }
}
