use crate::configuration::ConfigurationError;
use crate::configuration::PpsConfig;
use crate::gps::GpsType;

use serde::Deserialize;

use std::convert::TryFrom;
use std::time::Duration;

use tokio_serial::DataBits;
use tokio_serial::FlowControl;
use tokio_serial::Parity;
use tokio_serial::SerialPortBuilder;
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

impl GpsConfig {
    pub fn messages(&self) -> Vec<String> {
        match &self.messages {
            Some(m) => m.clone(),
            None => vec![],
        }
    }
}

impl TryFrom<GpsConfig> for SerialPortBuilder {
    type Error = ConfigurationError;

    fn try_from(device: GpsConfig) -> Result<SerialPortBuilder, ConfigurationError> {
        let path = device.device;
        let mut baud_rate = 38400;

        if let Some(b) = device.baud_rate {
            baud_rate = b;
        }

        let builder = tokio_serial::new(path, baud_rate);

        let mut data_bits = DataBits::Eight;
        let mut flow_control = FlowControl::None;
        let mut parity = Parity::None;
        let mut stop_bits = StopBits::One;
        let mut timeout = Duration::from_millis(1);

        if let Some(f) = device.framing {
            if f.len() != 3 {
                return Err(ConfigurationError::InvalidFraming(f));
            }
            let framing_data_bits = f.chars().next().unwrap();

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

        let builder = builder.data_bits(data_bits);
        let builder = builder.parity(parity);
        let builder = builder.stop_bits(stop_bits);

        if let Some(f) = device.flow_control {
            if f.len() != 1 {
                return Err(ConfigurationError::InvalidFlowControl(f));
            }

            let config_flow_control = f.chars().next().unwrap();

            flow_control = match config_flow_control {
                'H' => FlowControl::Hardware,
                'S' => FlowControl::Software,
                'N' => FlowControl::None,
                _ => return Err(ConfigurationError::InvalidFlowControl(f)),
            };
        }

        let builder = builder.flow_control(flow_control);

        if let Some(t) = device.timeout {
            timeout = Duration::from_millis(t.into());
        }

        let builder = builder.timeout(timeout);

        Ok(builder)
    }
}
