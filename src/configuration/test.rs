use crate::configuration::*;

use std::convert::TryFrom;
use std::fs;
use std::io;
use std::io::Write;
use std::time::Duration;

use tempfile::tempdir;
use tempfile::TempDir;

use tokio_serial::DataBits;
use tokio_serial::FlowControl;
use tokio_serial::Parity;
use tokio_serial::SerialPortSettings;
use tokio_serial::StopBits;

use tracing::Level;

use tracing_subscriber::filter::EnvFilter;

fn write(content: &str) -> Result<(fs::File, TempDir), io::Error> {
    let dir = tempdir()?;
    let path = dir.path().join("where.toml");

    let mut file = fs::File::create(path.clone())?;

    file.write_all(content.as_bytes())?;

    Ok((file, dir))
}

#[test]
fn test_config() {
    let (_, dir) = write(
        r#"
log_filter = "debug"

[[gps]]
name = "GPS0"
device = "/dev/gps0"
baud_rate = 38400
messages = [ "ZDA" ]
ntp_unit = 2

[gps.pps]
device = "/dev/pps0"
ntp_unit = 3

[[gps]]
name = "GPS1"
device = "/dev/gps1"

[gps.pps]
device = "/dev/pps1"
    "#,
    )
    .unwrap();

    let path = dir.path().join("where.toml");
    let config = Configuration::load(path).unwrap();

    let pps0 = PpsConfig {
        device: "/dev/pps0".to_string(),
        ntp_unit: Some(3),
    };

    let gps0 = GpsConfig {
        name: "GPS0".to_string(),
        device: "/dev/gps0".to_string(),
        pps: Some(pps0),
        baud_rate: Some(38400),
        framing: None,
        flow_control: None,
        timeout: None,
        messages: Some(vec!["ZDA".to_string()]),
        ntp_unit: Some(2),
    };

    let pps1 = PpsConfig {
        device: "/dev/pps1".to_string(),
        ntp_unit: None,
    };

    let gps1 = GpsConfig {
        name: "GPS1".to_string(),
        device: "/dev/gps1".to_string(),
        pps: Some(pps1),
        baud_rate: None,
        framing: None,
        flow_control: None,
        timeout: None,
        messages: None,
        ntp_unit: None,
    };

    let expected = Configuration {
        log_filter: Some(String::from("debug")),
        gps: vec![gps0, gps1],
        log_level: None,
    };

    assert_eq!(expected, config);
}

#[test]
fn test_try_from_gps_config() {
    let gps = GpsConfig {
        name: "GPS".to_string(),
        device: "/dev/gps0".to_string(),
        pps: None,
        baud_rate: Some(38400),
        framing: Some("7O2".to_string()),
        flow_control: Some("H".to_string()),
        timeout: Some(10),
        messages: None,
        ntp_unit: None,
    };

    let settings = SerialPortSettings::try_from(gps).unwrap();

    assert_eq!(38400, settings.baud_rate);
    assert_eq!(DataBits::Seven, settings.data_bits);
    assert_eq!(FlowControl::Hardware, settings.flow_control);
    assert_eq!(Parity::Odd, settings.parity);
    assert_eq!(StopBits::Two, settings.stop_bits);
    assert_eq!(Duration::from_millis(10), settings.timeout);
}

#[test]
fn test_try_from_gps_config_default() {
    let gps = GpsConfig {
        name: "GPS".to_string(),
        device: "/dev/gps0".to_string(),
        pps: None,
        baud_rate: None,
        framing: None,
        flow_control: None,
        timeout: None,
        messages: None,
        ntp_unit: None,
    };

    let settings = SerialPortSettings::try_from(gps).unwrap();

    assert_eq!(38400, settings.baud_rate);
    assert_eq!(DataBits::Eight, settings.data_bits);
    assert_eq!(FlowControl::None, settings.flow_control);
    assert_eq!(Parity::None, settings.parity);
    assert_eq!(StopBits::One, settings.stop_bits);
    assert_eq!(Duration::from_millis(1), settings.timeout);
}

#[test]
fn test_try_from_gps_config_error() {
    let gps = GpsConfig {
        name: "GPS".to_string(),
        device: "/dev/gps0".to_string(),
        pps: None,
        baud_rate: Some(38400),
        framing: Some("9N1".to_string()),
        flow_control: None,
        timeout: None,
        messages: None,
        ntp_unit: None,
    };

    match SerialPortSettings::try_from(gps).err().unwrap() {
        ConfigurationError::InvalidDataBits(e) => assert_eq!('9', e),
        _ => assert!(false),
    }
}

#[test]
fn test_try_from_log_filter_default() {
    let config = Configuration {
        log_filter: None,
        log_level: None,
        gps: vec![],
    };

    let filter = EnvFilter::try_from(config).unwrap();

    let expected = String::from("info");

    assert_eq!(expected, filter.to_string());
}

#[test]
fn test_try_from_log_filter_set() {
    let config = Configuration {
        log_filter: Some(String::from("trace")),
        log_level: None,
        gps: vec![],
    };

    let filter = EnvFilter::try_from(config).unwrap();

    let expected = String::from("trace");

    assert_eq!(expected, filter.to_string());
}

#[test]
fn test_try_from_log_filter_error() {
    let config = Configuration {
        log_filter: Some(String::from("=garbage")),
        log_level: None,
        gps: vec![],
    };

    match EnvFilter::try_from(config).err().unwrap() {
        ConfigurationError::InvalidLogFilter(f, e) => {
            assert_eq!("=garbage", f);
            assert_eq!("invalid filter directive", e.to_string());
        }
        _ => assert!(false),
    };
}

#[test]
fn test_from_tracing_none() {
    let config = Configuration {
        gps: vec![],
        log_filter: None,
        log_level: None,
    };

    assert_eq!(Level::INFO, Level::from(config));
}

#[test]
fn test_from_tracing_set() {
    let config = Configuration {
        gps: vec![],
        log_filter: None,
        log_level: Some("debug".to_string()),
    };

    assert_eq!(Level::DEBUG, Level::from(config));
}

#[test]
fn test_from_tracing_invalid() {
    let config = Configuration {
        gps: vec![],
        log_filter: None,
        log_level: Some("dabug".to_string()),
    };

    assert_eq!(Level::INFO, Level::from(config));
}
