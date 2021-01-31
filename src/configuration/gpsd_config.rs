use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GpsdConfig {
  pub bind_addresses: Vec<String>,
  pub port: u16,
}
