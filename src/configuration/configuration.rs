use crate::configuration::ConfigurationError;
use crate::configuration::GpsConfig;

use serde::Deserialize;

use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Configuration {
    pub gps: Vec<GpsConfig>,
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
