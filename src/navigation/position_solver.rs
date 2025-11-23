use crate::gps::ephemeris::Ephemeris;
use crate::navigation::{GnssPosition, Pseudorange};
use crate::{Error, Result};
use nalgebra::{DMatrix, DVector, Vector3, Vector4};

#[derive(Debug, Clone)]
pub struct PositionSolution {
    pub position: GnssPosition,
    pub residuals: Vec<f64>,
    pub iterations: usize,
    pub converged: bool,
}

pub struct PositionSolver {
    max_iterations: usize,
    convergence_threshold: f64,
}

impl Default for PositionSolver {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            convergence_threshold: 1.0, // 1 meter
        }
    }
}

impl PositionSolver {
    pub fn new(max_iterations: usize, convergence_threshold: f64) -> Self {
        Self {
            max_iterations,
            convergence_threshold,
        }
    }

    /// Solve for position using weighted least squares
    pub fn solve(
        &self,
        pseudoranges: &[Pseudorange],
        ephemerides: &[Ephemeris],
        gps_time: f64,
    ) -> Result<PositionSolution> {
        if pseudoranges.len() < 4 {
            return Err(Error::InsufficientSatellites(pseudoranges.len()));
        }

        // Match pseudoranges with ephemerides
        let mut measurements = Vec::new();
        for pr in pseudoranges {
            if let Some(eph) = ephemerides.iter().find(|e| e.prn == pr.prn) {
                measurements.push((pr, eph));
            }
        }

        if measurements.len() < 4 {
            return Err(Error::InsufficientSatellites(measurements.len()));
        }

        // Initial guess (center of Earth)
        let mut state = Vector4::new(0.0, 0.0, 0.0, 0.0); // [x, y, z, clock_bias]

        let mut converged = false;
        let mut iterations = 0;

        for iter in 0..self.max_iterations {
            iterations = iter + 1;

            // Build design matrix H and residual vector
            let n = measurements.len();
            let mut h_matrix = DMatrix::zeros(n, 4);
            let mut residuals = DVector::zeros(n);
            let mut weights = DVector::zeros(n);

            for (i, (pr, eph)) in measurements.iter().enumerate() {
                // Calculate satellite position
                let sat_pos = eph.satellite_position(gps_time);

                // Apply satellite clock correction
                let clock_corr = eph.clock_correction(gps_time);

                // Predicted range
                let user_pos = Vector3::new(state[0], state[1], state[2]);
                let range_vec = sat_pos - user_pos;
                let range = range_vec.norm();

                // Predicted pseudorange
                let predicted_pr = range + state[3] - clock_corr * crate::constants::SPEED_OF_LIGHT;

                // Residual
                residuals[i] = pr.pseudorange - predicted_pr;

                // Unit vector from user to satellite
                let unit = range_vec / range;

                // Design matrix row
                h_matrix[(i, 0)] = -unit.x;
                h_matrix[(i, 1)] = -unit.y;
                h_matrix[(i, 2)] = -unit.z;
                h_matrix[(i, 3)] = 1.0;

                // Weight based on CN0
                weights[i] = pr.weight();
            }

            // Weighted least squares: delta_x = (H^T W H)^-1 H^T W residuals
            let w_matrix = DMatrix::from_diagonal(&weights);
            let h_t = h_matrix.transpose();
            let htwh = &h_t * &w_matrix * &h_matrix;

            // Check if matrix is singular
            let htwh_inv = match htwh.try_inverse() {
                Some(inv) => inv,
                None => {
                    return Err(Error::SolverFailed("Singular matrix".to_string()));
                }
            };

            let delta = htwh_inv * &h_t * &w_matrix * &residuals;

            // Check convergence (before moving delta)
            let correction_norm = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();

            // Update state
            state += delta;

            if correction_norm < self.convergence_threshold {
                converged = true;
                break;
            }
        }

        if !converged {
            tracing::warn!("Position solver did not converge after {} iterations", iterations);
        }

        // Create position solution
        let ecef = Vector3::new(state[0], state[1], state[2]);
        let mut position = GnssPosition::from_ecef(ecef, state[3], measurements.len());
        position.time = gps_time;

        // Calculate DOP values
        let (hdop, vdop, pdop) = self.calculate_dop(&measurements, &state, gps_time);
        position.hdop = hdop;
        position.vdop = vdop;
        position.pdop = pdop;

        // Calculate final residuals
        let residuals: Vec<f64> = measurements
            .iter()
            .map(|(pr, eph)| {
                let sat_pos = eph.satellite_position(gps_time);
                let user_pos = Vector3::new(state[0], state[1], state[2]);
                let range = (sat_pos - user_pos).norm();
                let clock_corr = eph.clock_correction(gps_time);
                let predicted = range + state[3] - clock_corr * crate::constants::SPEED_OF_LIGHT;
                pr.pseudorange - predicted
            })
            .collect();

        Ok(PositionSolution {
            position,
            residuals,
            iterations,
            converged,
        })
    }

    /// Calculate Dilution of Precision (DOP) values
    fn calculate_dop(
        &self,
        measurements: &[(&Pseudorange, &Ephemeris)],
        state: &Vector4<f64>,
        gps_time: f64,
    ) -> (f64, f64, f64) {
        let n = measurements.len();
        let mut h_matrix = DMatrix::zeros(n, 4);

        for (i, (_, eph)) in measurements.iter().enumerate() {
            let sat_pos = eph.satellite_position(gps_time);
            let user_pos = Vector3::new(state[0], state[1], state[2]);
            let range_vec = sat_pos - user_pos;
            let range = range_vec.norm();
            let unit = range_vec / range;

            h_matrix[(i, 0)] = -unit.x;
            h_matrix[(i, 1)] = -unit.y;
            h_matrix[(i, 2)] = -unit.z;
            h_matrix[(i, 3)] = 1.0;
        }

        // DOP matrix: (H^T H)^-1
        let h_t = h_matrix.transpose();
        let hth = &h_t * &h_matrix;

        if let Some(q) = hth.try_inverse() {
            let hdop = (q[(0, 0)] + q[(1, 1)]).sqrt();
            let vdop = q[(2, 2)].sqrt();
            let pdop = (q[(0, 0)] + q[(1, 1)] + q[(2, 2)]).sqrt();
            (hdop, vdop, pdop)
        } else {
            (99.99, 99.99, 99.99) // Invalid DOP
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_creation() {
        let solver = PositionSolver::default();
        assert_eq!(solver.max_iterations, 10);
    }

    #[test]
    fn test_insufficient_satellites() {
        let solver = PositionSolver::default();
        let prs = vec![
            Pseudorange::from_tracking(1, 0.0, 1.023e6, 0.0, 0.0, 40.0, 100),
            Pseudorange::from_tracking(2, 0.0, 1.023e6, 0.0, 0.0, 40.0, 100),
        ];
        let ephs = vec![];

        let result = solver.solve(&prs, &ephs, 0.0);
        assert!(result.is_err());
    }
}
