use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("data bits {0} must be 8, 7, 6, or 5")]
    InvalidDataBits(char),
    #[error("flow control {0} must be H(ardware), S(oftware), or N(one)")]
    InvalidFlowControl(String),
    #[error("framing {0} must be three characters, data bits, parity, stop bits")]
    InvalidFraming(String),
    #[error("log filter {0} is invalid: {1}")]
    InvalidLogFilter(String, tracing_subscriber::filter::ParseError),
    #[error("parity {0} must be N(one), O(dd), or E(ven)")]
    InvalidParity(char),
    #[error("parity {0} must be 1 or 2")]
    InvalidStopBits(char),
    #[error("invalid configuration file: {0}")]
    De(#[from] toml::de::Error),
    #[error("unable to read configuration file: {0}")]
    Io(#[from] std::io::Error),
}
