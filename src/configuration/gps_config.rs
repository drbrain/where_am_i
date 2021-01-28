use crate::configuration::ConfigurationError;
use crate::configuration::PpsConfig;
use crate::gps::GpsType;

use serde::Deserialize;

use std::convert::TryFrom;
use std::time::Duration;

use tokio_serial::DataBits;
use tokio_serial::FlowControl;
use tokio_serial::Parity;
use tokio_serial::SerialPortSettings;
use tokio_serial::StopBits;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GpsConfig {
    pub name: String,
    pub device: String,
    pub gps_type: GpsType,
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
