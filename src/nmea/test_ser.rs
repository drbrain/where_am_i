use crate::nmea::*;

#[test]
fn test_position_poll() {
    let poll = UBXPositionPoll {};

    let nmea = ser::to_string(&poll).unwrap();

    assert_eq!(String::from("PUBX,00"), nmea);
}

#[test]
fn test_svs_poll() {
    let poll = UBXSvsPoll {};

    let nmea = ser::to_string(&poll).unwrap();

    assert_eq!(String::from("PUBX,03"), nmea);
}

#[test]
fn test_time_poll() {
    let poll = UBXTimePoll {};

    let nmea = ser::to_string(&poll).unwrap();

    assert_eq!(String::from("PUBX,04"), nmea);
}

#[test]
fn test_rate() {
    let rate = UBXRate { message: "GLL".to_string(), rddc: 1, rus1: 0, rus2: 0, rusb: 0, rspi: 0, reserved: 0 };

    let nmea = ser::to_string(&rate).unwrap();

    assert_eq!(String::from("PUBX,40,GLL,1,0,0,0,0,0"), nmea);

    let rate = UBXRate { message: "ZDA".to_string(), rddc: 0, rus1: 1, rus2: 0, rusb: 0, rspi: 0, reserved: 0 };

    let nmea = ser::to_string(&rate).unwrap();

    assert_eq!(String::from("PUBX,40,ZDA,0,1,0,0,0,0"), nmea);
}

#[test]
fn test_config() {
    let config = UBXConfig { port: UBXPort::USART1, in_proto: parser::UBXPortMask::USB | parser::UBXPortMask::SPI, out_proto: parser::UBXPortMask::USB, baudrate: 19200, autobauding: false };

    let nmea = ser::to_string(&config).unwrap();

    assert_eq!(String::from("PUBX,41,1,0007,0003,19200,0"), nmea);

}
