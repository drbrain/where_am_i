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

[gps.pps]
device = "/dev/pps0"

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

    let pps0 = Pps {
        device: "/dev/pps0".to_string(),
    };

    let gps0 = Gps {
        name: "GPS0".to_string(),
        device: "/dev/gps0".to_string(),
        pps: Some(pps0),
        baud_rate: Some(38400),
        framing: None,
        flow_control: None,
        timeout: None,
        messages: Some(vec!["ZDA".to_string()]),
    };

    let pps1 = Pps {
        device: "/dev/pps1".to_string(),
    };

    let gps1 = Gps {
        name: "GPS1".to_string(),
        device: "/dev/gps1".to_string(),
        pps: Some(pps1),
        baud_rate: None,
        framing: None,
        flow_control: None,
        timeout: None,
        messages: None,
    };

    let expected = Configuration {
        gps: vec![gps0, gps1],
    };

    assert_eq!(expected, config);
}
