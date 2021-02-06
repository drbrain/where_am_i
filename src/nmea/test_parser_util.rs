use crate::nmea::parser_util::*;

#[test]
fn test_comma() {
    assert_eq!(",", parser::comma::<VE>(",").unwrap().1);
}

#[test]
fn test_date_invalid() {
    let error = parser::date::<VE>("890620").err().unwrap();

    assert_eq!(
        "Parsing Error: VerboseError { errors: [(\"890620\", Nom(MapOpt))] }",
        error.to_string()
    );
}

#[test]
fn test_lat() {
    assert_eq!(47.28521118, parser::lat::<VE>("4717.112671").unwrap().1);
}

#[test]
fn test_latlon() {
    let lat_lon = parser::latlon::<VE>("4717.11399,N,00833.91590,W")
        .unwrap()
        .1
        .unwrap();

    assert_eq!(47.285233, lat_lon.latitude);
    assert_eq!(-8.565265, lat_lon.longitude);
}

#[test]
fn test_latlon_empty() {
    let lat_lon = parser::latlon::<VE>(",,,,").unwrap().1;

    assert_eq!(None, lat_lon);
}

#[test]
fn test_lon() {
    assert_eq!(8.56524738, parser::lon::<VE>("00833.914843").unwrap().1);
}

#[test]
fn test_time_hms() {
    let input = "072732";
    let result = parser::time::<VE>(input);

    let time = p::<NaiveTime>(input, result);

    assert_eq!(NaiveTime::from_hms_milli(7, 27, 32, 0), time);
}

#[test]
fn test_time_hms_centi() {
    let input = "072732.91";
    let result = parser::time::<VE>(input);

    let time = p::<NaiveTime>(input, result);

    assert_eq!(NaiveTime::from_hms_milli(7, 27, 32, 910), time);
}

#[test]
fn test_time_hms_milli() {
    let input = "072732.911";
    let result = parser::time::<VE>(input);

    let time = p::<NaiveTime>(input, result);

    assert_eq!(NaiveTime::from_hms_milli(7, 27, 32, 911), time);
}

#[test]
fn test_time_invalid() {
    let error = parser::time::<VE>("811118").err().unwrap();

    assert_eq!(
        "Parsing Error: VerboseError { errors: [(\"811118\", Nom(MapOpt))] }",
        error.to_string()
    );
}

