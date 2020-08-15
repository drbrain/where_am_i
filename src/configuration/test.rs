use crate::configuration::*;

use std::io::Write;

use tempfile::tempdir;
use tempfile::TempDir;

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
        gps: vec![gps0, gps1],
    };

    assert_eq!(expected, config);
}

#[test]
fn test_try_from() {
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
fn test_try_from_default() {
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
fn test_try_from_error() {
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
