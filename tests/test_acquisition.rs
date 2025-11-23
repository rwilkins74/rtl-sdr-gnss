use rtl_sdr_gnss::gps::ca_code::CaCodeGenerator;
use rtl_sdr_gnss::signal::fft_correlate;
use rtl_sdr_gnss::constants::GPS_CA_CHIPPING_RATE;
use num_complex::Complex;

#[test]
fn test_ca_code_autocorrelation() {
    // Test that C/A code has good autocorrelation properties
    let sample_rate = 2_048_000.0;
    let prn = 1;

    // Generate C/A code
    let code = CaCodeGenerator::generate_sampled(prn, sample_rate, GPS_CA_CHIPPING_RATE);
    let samples_per_code = code.len();

    println!("PRN {} code has {} samples", prn, samples_per_code);

    // Create signal that's just the code (no noise, no carrier)
    let signal: Vec<Complex<f32>> = code.iter()
        .map(|&c| Complex::new(c, 0.0))
        .collect();

    // Correlate with itself
    let correlation = fft_correlate(&signal, &code);

    // Find peak
    let (max_idx, &max_val) = correlation.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();

    // Calculate mean of non-peak values
    let mean_val: f32 = correlation.iter()
        .enumerate()
        .filter(|(i, _)| *i != max_idx)
        .map(|(_, v)| v)
        .sum::<f32>() / (correlation.len() - 1) as f32;

    let peak_to_mean = max_val / mean_val;

    println!("Autocorrelation: peak={:.2} at index {}, mean={:.4}, ratio={:.2}",
             max_val, max_idx, mean_val, peak_to_mean);

    // For perfect autocorrelation with no noise, we should see peak >> mean
    // GPS C/A codes have peak-to-mean ratio of about 1023 (code length)
    // With our normalization, we might see different values
    assert!(peak_to_mean > 10.0, "Autocorrelation peak should be much higher than mean");
    assert_eq!(max_idx, 0, "Peak should be at zero offset for autocorrelation");
}

#[test]
fn test_ca_code_cross_correlation() {
    // Test that different PRN codes have low cross-correlation
    let sample_rate = 2_048_000.0;

    let code1 = CaCodeGenerator::generate_sampled(1, sample_rate, GPS_CA_CHIPPING_RATE);
    let code2 = CaCodeGenerator::generate_sampled(2, sample_rate, GPS_CA_CHIPPING_RATE);

    // Create signal from PRN 1
    let signal: Vec<Complex<f32>> = code1.iter()
        .map(|&c| Complex::new(c, 0.0))
        .collect();

    // Correlate with PRN 2
    let correlation = fft_correlate(&signal, &code2);

    let max_val = correlation.iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let mean_val: f32 = correlation.iter().sum::<f32>() / correlation.len() as f32;
    let peak_to_mean = max_val / mean_val;

    println!("Cross-correlation PRN1 vs PRN2: peak={:.4}, mean={:.4}, ratio={:.2}",
             max_val, mean_val, peak_to_mean);

    // Cross-correlation should be low
    assert!(peak_to_mean < 5.0, "Cross-correlation should be low for different PRNs");
}
