use serde::Deserialize;

use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use tokio_serial::DataBits;
use tokio_serial::FlowControl;
use tokio_serial::Parity;
use tokio_serial::SerialPortSettings;
use tokio_serial::StopBits;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Configuration {
    pub gps: Vec<GpsConfig>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GpsConfig {
    pub name: String,
    pub device: String,
    pub pps: Option<PpsConfig>,
    pub baud_rate: Option<u32>,
    pub framing: Option<String>,
    pub flow_control: Option<String>,
    pub timeout: Option<u32>,
    pub messages: Option<Vec<String>>,
    pub ntp_unit: Option<i32>,
}

impl TryFrom<GpsConfig> for SerialPortSettings {
    type Error = ConfigurationError;

    fn try_from(device: GpsConfig) -> Result<SerialPortSettings, ConfigurationError> {
        let mut baud_rate = 38400;
        let mut data_bits = DataBits::Eight;
        let mut flow_control = FlowControl::None;
        let mut parity = Parity::None;
        let mut stop_bits = StopBits::One;
        let mut timeout = Duration::from_millis(1);

        if let Some(b) = device.baud_rate {
            baud_rate = b;
        }

        if let Some(f) = device.framing {
            if f.len() != 3 {
                return Err(ConfigurationError::InvalidFraming(f));
            }
            let framing_data_bits = f.chars().nth(0).unwrap();

            data_bits = match framing_data_bits {
                '8' => DataBits::Eight,
                '7' => DataBits::Seven,
                '6' => DataBits::Six,
                '5' => DataBits::Five,
                _ => return Err(ConfigurationError::InvalidDataBits(framing_data_bits)),
            };

            let framing_parity = f.chars().nth(1).unwrap();

            parity = match framing_parity {
                'N' => Parity::None,
                'O' => Parity::Odd,
                'E' => Parity::Even,
                _ => return Err(ConfigurationError::InvalidParity(framing_parity)),
            };

            let framing_stop_bits = f.chars().nth(2).unwrap();

            stop_bits = match framing_stop_bits {
                '1' => StopBits::One,
                '2' => StopBits::Two,
                _ => return Err(ConfigurationError::InvalidStopBits(framing_stop_bits)),
            };
        };

        if let Some(f) = device.flow_control {
            if f.len() != 1 {
                return Err(ConfigurationError::InvalidFlowControl(f));
            }

            let config_flow_control = f.chars().nth(0).unwrap();

            flow_control = match config_flow_control {
                'H' => FlowControl::Hardware,
                'S' => FlowControl::Software,
                'N' => FlowControl::None,
                _ => return Err(ConfigurationError::InvalidFlowControl(f)),
            };
        }

        if let Some(t) = device.timeout {
            timeout = Duration::from_millis(t.into());
        }

        Ok(SerialPortSettings {
            baud_rate,
            data_bits,
            flow_control,
            parity,
            stop_bits,
            timeout,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PpsConfig {
    pub device: String,
    pub ntp_unit: Option<i32>,
}

impl Configuration {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Configuration, ConfigurationError> {
        let source = fs::read_to_string(path)?;

        parse(source)
    }
}

fn parse(source: String) -> Result<Configuration, ConfigurationError> {
    match toml::from_str(&source) {
        Err(e) => Err(ConfigurationError::from(e)),
        Ok(c) => Ok(c),
    }
}

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

#[cfg(test)]
mod test;
