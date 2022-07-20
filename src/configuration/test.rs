use crate::configuration::*;
use crate::gps::GpsType;

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
use tokio_serial::SerialPortBuilder;
use tokio_serial::StopBits;

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
gps_type = "ublox"
baud_rate = 38400
messages = [ "ZDA" ]
ntp_unit = 2

[gps.pps]
device = "/dev/pps0"
ntp_unit = 3

[[gps]]
name = "GPS1"
device = "/dev/gps1"
gps_type = "generic"

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
        gps_type: GpsType::UBlox,
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
        gps_type: GpsType::Generic,
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
        gpsd: None,
        prometheus: None,
    };

    assert_eq!(expected, config);
}

#[test]
fn test_config_gpsd() {
    let (_, dir) = write(
        r#"
log_filter = "debug"

[gpsd]
bind_addresses = ["127.0.0.1"]
port = 2947

[[gps]]
name = "GPS0"
device = "/dev/gps0"
gps_type = "generic"
baud_rate = 38400
messages = [ "ZDA" ]
ntp_unit = 2

    "#,
    )
    .unwrap();

    let path = dir.path().join("where.toml");
    let config = Configuration::load(path).unwrap();

    let gps0 = GpsConfig {
        name: "GPS0".to_string(),
        device: "/dev/gps0".to_string(),
        gps_type: GpsType::Generic,
        pps: None,
        baud_rate: Some(38400),
        framing: None,
        flow_control: None,
        timeout: None,
        messages: Some(vec!["ZDA".to_string()]),
        ntp_unit: Some(2),
    };

    let gpsd = GpsdConfig {
        bind_addresses: vec!["127.0.0.1".to_string()],
        port: 2947,
    };

    let expected = Configuration {
        log_filter: Some(String::from("debug")),
        gps: vec![gps0],
        gpsd: Some(gpsd),
        prometheus: None,
    };

    assert_eq!(expected, config);
}

#[test]
fn test_try_from_serial_port_settings() {
    let gps = GpsConfig {
        name: "GPS".to_string(),
        device: "/dev/gps0".to_string(),
        gps_type: GpsType::Generic,
        pps: None,
        baud_rate: Some(38400),
        framing: Some("7O2".to_string()),
        flow_control: Some("H".to_string()),
        timeout: Some(10),
        messages: None,
        ntp_unit: None,
    };

    let settings = SerialPortBuilder::try_from(gps).unwrap();

    let expected = tokio_serial::new("/dev/gps0", 38400);
    let expected = expected.data_bits(DataBits::Seven);
    let expected = expected.flow_control(FlowControl::Hardware);
    let expected = expected.parity(Parity::Odd);
    let expected = expected.stop_bits(StopBits::Two);
    let expected = expected.timeout(Duration::from_millis(10));

    assert_eq!(expected, settings);
}

#[test]
fn test_try_from_serial_port_settings_default() {
    let gps = GpsConfig {
        name: "GPS".to_string(),
        device: "/dev/gps0".to_string(),
        gps_type: GpsType::Generic,
        pps: None,
        baud_rate: None,
        framing: None,
        flow_control: None,
        timeout: None,
        messages: None,
        ntp_unit: None,
    };

    let settings = SerialPortBuilder::try_from(gps).unwrap();

    let expected = tokio_serial::new("/dev/gps0", 38400);
    let expected = expected.data_bits(DataBits::Eight);
    let expected = expected.flow_control(FlowControl::None);
    let expected = expected.parity(Parity::None);
    let expected = expected.stop_bits(StopBits::One);
    let expected = expected.timeout(Duration::from_millis(1));

    assert_eq!(expected, settings);
}

#[test]
fn test_try_from_serial_port_settings_error() {
    let gps = GpsConfig {
        name: "GPS".to_string(),
        device: "/dev/gps0".to_string(),
        gps_type: GpsType::Generic,
        pps: None,
        baud_rate: Some(38400),
        framing: Some("9N1".to_string()),
        flow_control: None,
        timeout: None,
        messages: None,
        ntp_unit: None,
    };

    match SerialPortBuilder::try_from(gps).err().unwrap() {
        ConfigurationError::InvalidDataBits(e) => assert_eq!('9', e),
        _ => assert!(false),
    }
}

#[test]
fn test_try_from_log_filter_default() {
    let config = Configuration {
        log_filter: None,
        gps: vec![],
        gpsd: None,
        prometheus: None,
    };

    let filter = EnvFilter::try_from(config).unwrap();

    let expected = String::from("info");

    assert_eq!(expected, filter.to_string());
}

#[test]
fn test_try_from_log_filter_set() {
    let config = Configuration {
        log_filter: Some(String::from("trace")),
        gps: vec![],
        gpsd: None,
        prometheus: None,
    };

    let filter = EnvFilter::try_from(config).unwrap();

    let expected = String::from("trace");

    assert_eq!(expected, filter.to_string());
}

#[test]
fn test_try_from_log_filter_error() {
    let config = Configuration {
        log_filter: Some(String::from("=garbage")),
        gps: vec![],
        gpsd: None,
        prometheus: None,
    };

    match EnvFilter::try_from(config).err().unwrap() {
        ConfigurationError::InvalidLogFilter(f, e) => {
            assert_eq!("=garbage", f);
            assert_eq!("invalid filter directive", e.to_string());
        }
        _ => assert!(false),
    };
}
