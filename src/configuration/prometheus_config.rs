use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PrometheusConfig {
    pub bind_addresses: Vec<String>,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            bind_addresses: vec!["127.0.0.1:9947".to_string(), "::1:9947".to_string()],
        }
    }
}
