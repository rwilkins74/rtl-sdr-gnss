use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("RTL-SDR error: {0}")]
    RtlSdr(String),

    #[error("Signal acquisition failed for PRN {0}")]
    AcquisitionFailed(u8),

    #[error("Tracking lost for PRN {0}")]
    TrackingLost(u8),

    #[error("Navigation message decode error: {0}")]
    NavMessageDecode(String),

    #[error("Insufficient satellites for position fix (need 4, have {0})")]
    InsufficientSatellites(usize),

    #[error("Position solver failed: {0}")]
    SolverFailed(String),

    #[error("SBAS decode error: {0}")]
    SbasDecode(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
