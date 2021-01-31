use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GpsdConfig {
  pub bind_address: Vec<String>,
  pub port: u16,
}
