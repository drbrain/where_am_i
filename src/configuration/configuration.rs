use crate::configuration::ConfigurationError;
use crate::configuration::GpsConfig;

use serde::Deserialize;

use std::convert::TryFrom;
use std::fs;
use std::path::Path;

use tracing_subscriber::filter::EnvFilter;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Configuration {
    pub log_filter: Option<String>,
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

impl TryFrom<Configuration> for EnvFilter {
    type Error = ConfigurationError;

    fn try_from(configuration: Configuration) -> Result<EnvFilter, ConfigurationError> {
        match configuration.log_filter {
            Some(f) => match EnvFilter::try_new(f.clone()) {
                Ok(f) => Ok(f),
                Err(e) => Err(ConfigurationError::InvalidLogFilter(f, e)),
            },
            None => Ok(EnvFilter::new("info")),
        }
    }
}
