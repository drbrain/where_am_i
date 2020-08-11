use serde::Deserialize;

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Configuration {
    pub gps: Vec<Gps>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Gps {
    pub name: String,
    pub device: String,
    pub pps: Option<Pps>,
    pub baud_rate: Option<u32>,
    pub framing: Option<String>,
    pub flow_control: Option<String>,
    pub timeout: Option<u32>,
    pub messages: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Pps {
    pub device: String,
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
    Box(Box<dyn Error>),
    De(toml::de::Error),
    Io(io::Error),
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigurationError::Box(e) => write!(f, "{}", e),
            ConfigurationError::De(e) => write!(f, "{}", e),
            ConfigurationError::Io(e) => write!(f, "{}", e),
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
