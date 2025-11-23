# RTL-SDR GNSS Receiver

A GPS receiver application built from scratch in Rust using RTL-SDR as the RF frontend. This project implements the complete GPS signal processing chain, from raw I/Q samples to position calculation with SBAS corrections.

## Features

- **Complete GPS L1 C/A Signal Processing**
  - Gold code generation for GPS satellites (PRN 1-32)
  - FFT-based signal acquisition with Doppler search
  - Tracking loops (DLL/PLL/FLL) for code and carrier
  - Navigation message decoding
  - Ephemeris and almanac parsing

- **Position Calculation**
  - Pseudorange measurement
  - Weighted least squares position solver
  - Satellite position computation from ephemeris
  - DOP (Dilution of Precision) calculation
  - Ionospheric and tropospheric corrections

- **SBAS Support**
  - SBAS signal acquisition (PRN 120-158)
  - SBAS message decoding
  - Fast and long-term corrections
  - Ionospheric grid corrections

- **Output Formats**
  - **NMEA 0183**: GGA, RMC, GSA, GSV, VTG sentences
  - **JSON**: Structured position and satellite data
  - **PPS**: Pulse-per-second output synchronized to GPS time

- **User Interface**
  - Real-time TUI showing satellite status, signal strength, and position
  - Headless mode for integration with other systems

## Architecture

### Module Structure

```
rtl-sdr-gnss/
├── src/
│   ├── main.rs              # Entry point and CLI
│   ├── lib.rs               # Library exports
│   ├── config.rs            # Configuration management
│   ├── constants.rs         # GPS constants and parameters
│   ├── error.rs             # Error types
│   ├── receiver.rs          # Main GNSS receiver orchestrator
│   │
│   ├── sdr/                 # SDR interface
│   │   ├── mod.rs           # SDR source trait
│   │   └── rtlsdr_source.rs # RTL-SDR implementation
│   │
│   ├── gps/                 # GPS signal processing
│   │   ├── mod.rs
│   │   ├── ca_code.rs       # C/A code generator
│   │   ├── acquisition.rs   # Signal acquisition
│   │   ├── tracking.rs      # Tracking channels (DLL/PLL)
│   │   ├── nav_message.rs   # Navigation message decoder
│   │   └── ephemeris.rs     # Ephemeris and satellite position
│   │
│   ├── signal/              # DSP utilities
│   │   ├── mod.rs
│   │   ├── correlator.rs    # E/P/L correlators
│   │   ├── discriminator.rs # Phase/frequency discriminators
│   │   └── loop_filter.rs   # Loop filters
│   │
│   ├── navigation/          # Navigation solution
│   │   ├── mod.rs
│   │   ├── pseudorange.rs   # Pseudorange calculation
│   │   ├── position_solver.rs # Least squares solver
│   │   └── corrections.rs   # Iono/tropo corrections
│   │
│   ├── sbas/                # SBAS processing
│   │   ├── mod.rs
│   │   ├── decoder.rs       # SBAS message decoder
│   │   └── corrections.rs   # SBAS corrections
│   │
│   ├── output/              # Output formats
│   │   ├── mod.rs
│   │   ├── nmea.rs          # NMEA sentence generation
│   │   ├── json.rs          # JSON output
│   │   └── pps.rs           # PPS generator
│   │
│   └── ui/                  # User interface
│       └── mod.rs           # TUI implementation
```

### Signal Processing Pipeline

1. **RF Frontend (RTL-SDR)**
   - Receives GPS L1 signals at 1575.42 MHz
   - Samples at 2.048 MHz (configurable)
   - Outputs I/Q samples

2. **Acquisition**
   - Searches for satellites across Doppler and code phase space
   - Uses FFT-based correlation for efficiency
   - Detects signals above threshold (default 2.5σ)

3. **Tracking**
   - **DLL (Delay Lock Loop)**: Tracks code phase
   - **PLL (Phase Lock Loop)**: Tracks carrier phase
   - **FLL (Frequency Lock Loop)**: Tracks carrier frequency
   - Extracts navigation data bits (50 bps)

4. **Navigation Message Decoding**
   - Syncs to 20ms navigation bits
   - Finds preambles and decodes subframes
   - Extracts ephemeris, almanac, and timing data

5. **Position Calculation**
   - Computes pseudoranges from code phase and TOW
   - Calculates satellite positions from ephemeris
   - Solves for user position using weighted least squares
   - Applies corrections (clock, ionosphere, troposphere)

6. **Output Generation**
   - Formats position as NMEA sentences
   - Generates JSON for programmatic access
   - Produces PPS signal aligned to GPS seconds

## Prerequisites

- **Hardware**: RTL-SDR dongle (RTL2832U-based)
- **Antenna**: GPS antenna with LNA (required for weak GPS signals)
- **Rust**: Version 1.70 or later
- **Operating System**: Linux (primary), macOS, Windows

### System Dependencies

```bash
# Ubuntu/Debian
sudo apt-get install libusb-1.0-0-dev pkg-config

# RTL-SDR library
sudo apt-get install librtlsdr-dev

# Or build from source:
git clone https://github.com/osmocom/rtl-sdr.git
cd rtl-sdr
mkdir build && cd build
cmake ../ -DINSTALL_UDEV_RULES=ON
make
sudo make install
sudo ldconfig
```

## Installation

```bash
# Clone the repository
git clone https://github.com/rwilkins74/rtl-sdr-gnss.git
cd rtl-sdr-gnss

# Build the project
cargo build --release

# Run tests
cargo test

# Install (optional)
cargo install --path .
```

## Usage

### Basic Usage

```bash
# Run with TUI (default)
./target/release/rtl-sdr-gnss

# Run in headless mode (output to stdout)
./target/release/rtl-sdr-gnss --no-ui

# Specify device index
./target/release/rtl-sdr-gnss --device 0

# Enable verbose logging
./target/release/rtl-sdr-gnss --verbose

# Use custom configuration
./target/release/rtl-sdr-gnss --config my_config.toml
```

### Output Redirection

```bash
# Save NMEA to file
./target/release/rtl-sdr-gnss --no-ui --nmea-output gps.nmea

# Save JSON to file
./target/release/rtl-sdr-gnss --no-ui --json-output gps.json

# Pipe to gpsd
./target/release/rtl-sdr-gnss --no-ui | gpsd -N /dev/stdin
```

### Configuration

Create a `config.toml` file:

```toml
[sdr]
sample_rate = 2048000
center_freq = 1575420000
gain = "auto"  # or a specific value like 40.0
freq_correction = 0

[acquisition]
coherent_integration_ms = 1
max_doppler_hz = 10000.0
doppler_step_hz = 500.0
threshold = 2.5

[tracking]
dll_bandwidth_hz = 2.0
pll_bandwidth_hz = 25.0
fll_bandwidth_hz = 5.0
integration_ms = 1
early_late_spacing = 0.5
min_cn0_db_hz = 25.0

[navigation]
min_satellites = 4
update_rate_hz = 1.0
use_iono_correction = true
use_tropo_correction = true

[sbas]
enabled = true
prns = [131, 133, 135, 138]  # WAAS satellites

[output]
nmea_enabled = true
nmea_rate_hz = 1.0
json_enabled = true
json_rate_hz = 1.0
pps_enabled = true
```

## Performance Tuning

### Acquisition

- **Doppler Search Range**: Adjust `max_doppler_hz` based on receiver dynamics
  - Stationary: ±5 kHz is usually sufficient
  - Moving vehicle: ±10 kHz
  - Aircraft: ±15 kHz or more

- **Threshold**: Adjust `threshold` for acquisition sensitivity
  - Lower threshold (2.0): More sensitive, more false positives
  - Higher threshold (3.0): Less sensitive, fewer false positives

### Tracking

- **Loop Bandwidths**: Trade-off between noise rejection and dynamics
  - **DLL**: 1-4 Hz (2 Hz is typical)
  - **PLL**: 15-30 Hz (25 Hz is typical)
  - **FLL**: 3-10 Hz (5 Hz is typical)

- **Integration Time**: Longer integration improves sensitivity but reduces dynamics
  - 1 ms: Standard, good for most applications
  - 5-20 ms: Better sensitivity for weak signals, slower convergence

### RTL-SDR Tuning

- **Sample Rate**: Higher rates give better time resolution
  - 2.048 MHz: Standard GPS (2 samples per chip)
  - 4.096 MHz: Better resolution, more CPU usage

- **Gain**: Balance between signal strength and saturation
  - Auto gain: Good starting point
  - Manual (20-40 dB): Fine-tune based on antenna and environment

## Troubleshooting

### No Satellites Acquired

1. **Check antenna connection**: GPS antenna must have clear view of sky
2. **Verify RTL-SDR**: Test with `rtl_test` utility
3. **Check frequency**: GPS L1 is at 1575.42 MHz
4. **Adjust gain**: Try different gain settings
5. **Increase threshold**: Try lowering acquisition threshold to 2.0

### Satellites Tracked but No Position

1. **Wait for ephemeris**: Takes 30 seconds to decode from each satellite
2. **Need 4+ satellites**: Position requires at least 4 satellites
3. **Check DOP**: High DOP (>20) indicates poor geometry

### Poor Position Accuracy

1. **Enable corrections**: Ensure ionospheric/tropospheric corrections are enabled
2. **Use SBAS**: Enable SBAS for differential corrections
3. **Check CN0**: Low signal strength (<30 dB-Hz) degrades accuracy
4. **Multipath**: Move away from buildings/reflective surfaces

## Development and Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test gps::ca_code

# Run with logging
RUST_LOG=debug cargo test
```

### Code Generation Test

```bash
# Verify C/A code generation
cargo test test_ca_code_length
cargo test test_different_prns_different_codes
```

### Benchmarking

```bash
# Build with optimizations
cargo build --release

# Profile acquisition performance
cargo test --release -- --nocapture test_acquisition
```

## Technical Details

### GPS Signal Characteristics

- **L1 Carrier**: 1575.42 MHz
- **C/A Code**: 1.023 MHz chipping rate, 1023 chips (1 ms period)
- **Navigation Message**: 50 bps, 1500 bits per subframe (30 seconds)
- **Satellite Orbit**: ~20,200 km altitude, 12-hour period

### Coordinate Systems

- **ECEF**: Earth-Centered Earth-Fixed (used internally)
- **LLA**: Latitude, Longitude, Altitude (output format)
- **WGS84**: World Geodetic System 1984 (reference ellipsoid)

### Accuracy Expectations

- **Cold Start**: 30-60 seconds to first fix
- **Position Accuracy**:
  - Without SBAS: 5-15 meters (95% CEP)
  - With SBAS: 1-3 meters (95% CEP)
- **Timing Accuracy**:
  - Without corrections: ~100 ns
  - With corrections: ~50 ns

## Known Limitations

1. **Ephemeris Parsing**: Simplified implementation (production needs full ICD-200 compliance)
2. **SBAS Decoding**: Basic framework (needs full message type support)
3. **Multi-GNSS**: GPS only (no GLONASS, Galileo, BeiDou support yet)
4. **Carrier Phase**: Not used for RTK (future enhancement)
5. **Integrity**: No RAIM or FDE implementation

## Future Enhancements

- [ ] Full GPS ICD-200 compliance for ephemeris parsing
- [ ] Complete SBAS message decoding (all message types)
- [ ] GLONASS L1OF support
- [ ] Galileo E1 support
- [ ] RTK (Real-Time Kinematic) positioning
- [ ] Kalman filter for position smoothing
- [ ] RAIM (Receiver Autonomous Integrity Monitoring)
- [ ] Multi-constellation support
- [ ] Hardware PPS output via GPIO

## Contributing

Contributions are welcome! Areas of interest:

- Improved signal acquisition algorithms
- Better loop filter implementations
- Full SBAS support
- Additional GNSS constellations
- Performance optimizations
- Documentation improvements

## References

- [GPS Interface Specification (IS-GPS-200)](https://www.gps.gov/technical/icwg/IS-GPS-200N.pdf)
- [Understanding GPS Principles and Applications, 2nd Edition](https://www.amazon.com/Understanding-Principles-Applications-Second-Kaplan/dp/1580538940)
- [RTCA DO-229D (SBAS MOPS)](https://standards.globalspec.com/std/1014192/rtca-do-229)
- [Kalman Filtering: Theory and Practice Using MATLAB](https://www.amazon.com/Kalman-Filtering-Practice-Using-MATLAB/dp/0470173661)

## License

This project is dual-licensed under MIT OR Apache-2.0.

## Acknowledgments

- The RTL-SDR community for the excellent SDR hardware and software
- Authors of GPS signal processing literature and open-source implementations
- Contributors to the Rust embedded and DSP ecosystems

## Contact

For questions, issues, or contributions, please open an issue on GitHub.

---

**Disclaimer**: This is an educational and experimental GPS receiver. For safety-critical or navigation-critical applications, use certified commercial GPS receivers.
