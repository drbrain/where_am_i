use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GpsdConfig {
    pub bind_addresses: Vec<String>,
    pub port: u16,
}

impl Default for GpsdConfig {
    fn default() -> Self {
        GpsdConfig {
            bind_addresses: vec!["127.0.0.1".to_string(), "::1".to_string()],
            port: 2947,
        }
    }
}
