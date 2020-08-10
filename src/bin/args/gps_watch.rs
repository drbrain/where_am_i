use argh::FromArgs;

use std::time::Duration;

use tokio_serial::DataBits;
use tokio_serial::FlowControl;
use tokio_serial::Parity;
use tokio_serial::SerialPortSettings;
use tokio_serial::StopBits;

#[derive(FromArgs)]
/// GPS watch
struct Args {
    /// GPS baud rate
    #[argh(option, default = "default_baud()")]
    baud_rate: u32,

    /// GPS data bits
    #[argh(option, default = "default_bits()")]
    data_bits: u8,

    /// GPS parity
    #[argh(option, default = "default_parity()")]
    parity: String,

    /// GPS stop bits
    #[argh(option, default = "default_stop_bits()")]
    stop_bits: u8,

    /// GPS flow control
    #[argh(option, default = "default_flow_control()")]
    flow_control: String,

    /// gps_device
    #[argh(positional)]
    gps_device: String,

    /// enable GPS messages, defaults to all messages if unset
    #[argh(option)]
    message: Vec<String>,
}

fn default_baud() -> u32 {
    38400
}

fn default_bits() -> u8 {
    8
}

fn default_flow_control() -> String {
    "none".to_string()
}

fn default_parity() -> String {
    "none".to_string()
}

fn default_stop_bits() -> u8 {
    1
}

fn data_bits_from_int(i: u8) -> Result<DataBits, String> {
    match i {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        e => Err(format!("invalid data bits {}", e)),
    }
}

fn flow_control_from_str(s: String) -> Result<FlowControl, String> {
    match s.to_lowercase().as_str() {
        "n" => Ok(FlowControl::None),
        "none" => Ok(FlowControl::None),
        "hardware" => Ok(FlowControl::Hardware),
        "software" => Ok(FlowControl::Software),
        e => Err(format!("invalid flow control {}", e)),
    }
}

fn parity_from_str(s: String) -> Result<Parity, String> {
    match s.to_lowercase().as_str() {
        "e" => Ok(Parity::Even),
        "even" => Ok(Parity::Even),
        "n" => Ok(Parity::None),
        "none" => Ok(Parity::None),
        "o" => Ok(Parity::Odd),
        "odd" => Ok(Parity::Odd),
        e => Err(format!("invalid parity {}", e)),
    }
}

fn stop_bits_from_str(i: u8) -> Result<StopBits, String> {
    match i {
        1 => Ok(StopBits::One),
        2 => Ok(StopBits::Two),
        e => Err(format!("invalid stop bits {}", e)),
    }
}

pub fn gps_watch_args() -> (String, SerialPortSettings, Vec<String>) {
    let args: Args = argh::from_env();

    let s = SerialPortSettings {
        baud_rate: args.baud_rate,
        data_bits: data_bits_from_int(args.data_bits).unwrap(),
        flow_control: flow_control_from_str(args.flow_control).unwrap(),
        parity: parity_from_str(args.parity).unwrap(),
        stop_bits: stop_bits_from_str(args.stop_bits).unwrap(),
        timeout: Duration::from_millis(1),
    };

    (args.gps_device, s, args.message)
}
