use crate::configuration::GpsConfig;

use std::convert::From;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Device {
    pub class: String,
    pub path: Option<String>,
    pub bps: Option<u32>,
    pub parity: Option<String>,
    pub stopbits: Option<String>,
    pub native: Option<u64>,
}

impl From<&GpsConfig> for Device {
    fn from(config: &GpsConfig) -> Self {
        let mut parity = None;
        let mut stopbits = None;

        if let Some(f) = &config.framing {
            if f.len() == 3 {
                let framing_parity = f.chars().nth(1).unwrap();

                parity = match framing_parity {
                    'N' => Some("N".to_string()),
                    'O' => Some("O".to_string()),
                    'E' => Some("E".to_string()),
                    _ => unreachable!(),
                };

                let framing_stop_bits = f.chars().nth(2).unwrap();

                stopbits = match framing_stop_bits {
                    '1' => Some("1".to_string()),
                    '2' => Some("2".to_string()),
                    _ => unreachable!(),
                };
            }
        };

        let bps = match config.baud_rate {
            None => Some(38400),
            Some(baud_rate) => Some(baud_rate),
        };

        Device {
            class: "DEVICE".to_string(),
            path: Some(config.device.clone()),
            bps,
            parity,
            stopbits,
            native: Some(0),
        }
    }
}
