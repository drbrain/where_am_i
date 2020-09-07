use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ConfigurationError {
    InvalidDataBits(char),
    InvalidFraming(String),
    InvalidFlowControl(String),
    InvalidParity(char),
    InvalidStopBits(char),
    Box(Box<dyn Error>),
    De(toml::de::Error),
    Io(io::Error),
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigurationError::InvalidDataBits(d) => {
                write!(fmt, "data bits {} must be 8, 7, 6, or 5", d)
            }
            ConfigurationError::InvalidFraming(f) => write!(
                fmt,
                "framing {} must be three characters, data bits, parity, stop bits",
                f
            ),
            ConfigurationError::InvalidFlowControl(f) => write!(
                fmt,
                "flow control {} must be H(ardware), S(oftware), or N(one)",
                f
            ),
            ConfigurationError::InvalidParity(p) => {
                write!(fmt, "parity {} must be N(one), O(dd), or E(ven)", p)
            }
            ConfigurationError::InvalidStopBits(s) => write!(fmt, "parity {} must be 1 or 2", s),
            ConfigurationError::Box(e) => write!(fmt, "{}", e),
            ConfigurationError::De(e) => write!(fmt, "{}", e),
            ConfigurationError::Io(e) => write!(fmt, "{}", e),
        }
    }
}

impl From<Box<dyn Error>> for ConfigurationError {
    fn from(e: Box<dyn Error>) -> ConfigurationError {
        ConfigurationError::Box(e)
    }
}

impl From<toml::de::Error> for ConfigurationError {
    fn from(e: toml::de::Error) -> ConfigurationError {
        ConfigurationError::De(e)
    }
}

impl From<io::Error> for ConfigurationError {
    fn from(e: io::Error) -> ConfigurationError {
        ConfigurationError::Io(e)
    }
}

impl std::error::Error for ConfigurationError {}
