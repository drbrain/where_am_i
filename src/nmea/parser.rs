use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::*;
use nom::number::complete::*;
use nom::sequence::*;
use nom::IResult;

#[derive(Clone, Debug, PartialEq)]
pub enum NMEA {
    DTM(DTMDatum),
    GAQ,
    GBQ,
    GBS,
    GGA,
    GLL,
    GLQ,
    GNQ,
    GNS,
    GPQ,
    GRS,
    GSA,
    GST,
    GSV,
    RMC,
    TXT,
    VLW,
    VTG,
    ZDA,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Talker {
    GPS,
    GLONASS,
    Galileo,
    BeiDuo,
    Combination,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NorthSouth {
    North,
    South,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EastWest {
    East,
    West,
}

fn any<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, String, E> {
    let (input, matched) = take_while(|c| c != ',')(input)?;

    Ok((input, matched.to_string()))
}

fn checksum<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    let (input, checksum) = preceded(star, hex_digit1)(input)?;

    Ok((input, checksum.parse().unwrap()))
}

fn comma<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag(",")(input)
}

fn dollar<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("$")(input)
}

fn eol<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("\r\n")(input)
}

fn east_west<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, EastWest, E> {
    let (input, ew) = alt((char('E'), char('W')))(input)?;

    let ew = match ew {
        'E' => EastWest::East,
        'W' => EastWest::West,
        _ => panic!("Unhandled alternate {:?}", ew),
    };

    Ok((input, ew))
}

fn north_south<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, NorthSouth, E> {
    let (input, ns) = alt((char('N'), char('S')))(input)?;

    let ns = match ns {
        'N' => NorthSouth::North,
        'S' => NorthSouth::South,
        _ => panic!("Unhandled alternate {:?}", ns),
    };

    Ok((input, ns))
}

fn star<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    tag("*")(input)
}

fn talker<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Talker, E> {
    let (input, talker) = alt((tag("GA"), tag("GB"), tag("GL"), tag("GN"), tag("GP")))(input)?;

    let talker = match talker {
        "GA" => Talker::Galileo,
        "GB" => Talker::BeiDuo,
        "GL" => Talker::GLONASS,
        "GN" => Talker::Combination,
        "GP" => Talker::GPS,
        _ => panic!("Unhandled alternate {:?}", talker),
    };

    Ok((input, talker))
}

fn verify_checksum(data: &str, checksum: &str) -> bool {
    let checksum = u8::from_str_radix(checksum, 16).unwrap();

    checksum == data.bytes().fold(0, |cs, b| cs ^ b)
}

fn line<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    let (input, line) = preceded(dollar, terminated(take_while1(|c| c != '\r'), eol))(input)?;

    let (_, (nmea_line, _, checksum)) = tuple((take_while1(|c| c != '*'), star, hex_digit1))(line)?;

    verify(rest, |_: &str| verify_checksum(nmea_line, checksum))("")?;

    Ok((input, nmea_line))
}

#[derive(Clone, Debug, PartialEq)]
pub struct DTMDatum {
    pub talker: Talker,
    pub datum: String,
    pub sub_datum: String,
    pub lat: f32,
    pub north_south: NorthSouth,
    pub lon: f32,
    pub east_west: EastWest,
    pub alt: f32,
    pub ref_datum: String,
}

fn dtm<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, DTMDatum, E> {
    let (
        _,
        (
            talker,
            _,
            _,
            datum,
            _,
            sub_datum,
            _,
            lat,
            _,
            north_south,
            _,
            lon,
            _,
            east_west,
            _,
            alt,
            _,
            ref_datum,
        ),
    ) = tuple((
        talker,
        tag("DTM"),
        comma,
        any,
        comma,
        any,
        comma,
        recognize_float,
        comma,
        north_south,
        comma,
        recognize_float,
        comma,
        east_west,
        comma,
        recognize_float,
        comma,
        any,
    ))(input)?;

    let lat = lat.parse().unwrap();
    let lon = lon.parse().unwrap();
    let alt = alt.parse().unwrap();

    let dtm_datum = DTMDatum {
        talker,
        datum,
        sub_datum,
        lat,
        north_south,
        lon,
        east_west,
        alt,
        ref_datum,
    };

    Ok((input, dtm_datum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comma() {
        assert_eq!(",", comma::<()>(",").unwrap().1);
    }

    #[test]
    fn test_dollar() {
        assert_eq!("$", dollar::<()>("$").unwrap().1);
    }

    #[test]
    fn test_line() {
        let full_line = "$GPDTM,W84,,0.0,N,0.0,E,0.0,W84*6F\r\n";

        assert_eq!("GPDTM,W84,,0.0,N,0.0,E,0.0,W84", line::<()>(full_line).unwrap().1);
    }

    #[test]
    fn test_talker() {
        assert_eq!(Talker::Galileo, talker::<()>("GA").unwrap().1);
        assert_eq!(Talker::BeiDuo, talker::<()>("GB").unwrap().1);
        assert_eq!(Talker::GLONASS, talker::<()>("GL").unwrap().1);
        assert_eq!(Talker::Combination, talker::<()>("GN").unwrap().1);
        assert_eq!(Talker::GPS, talker::<()>("GP").unwrap().1);
    }

    #[test]
    fn test_dtm() {
        let parsed = dtm::<VerboseError<&str>>("GPDTM,W84,,0.0,N,0.0,E,0.0,W84").unwrap().1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_eq!("W84".to_string(), parsed.datum);
        assert_eq!("".to_string(), parsed.sub_datum);
        assert_approx_eq!(0.0, parsed.lat);
        assert_eq!(NorthSouth::North, parsed.north_south);
        assert_approx_eq!(0.0, parsed.lon);
        assert_eq!(EastWest::East, parsed.east_west);
        assert_approx_eq!(0.0, parsed.alt);
        assert_eq!("W84".to_string(), parsed.ref_datum);

        let parsed = dtm::<VerboseError<&str>>("GPDTM,999,,0.08,N,0.07,E,-47.7,W84").unwrap().1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_eq!("999".to_string(), parsed.datum);
        assert_eq!("".to_string(), parsed.sub_datum);
        assert_approx_eq!(0.08, parsed.lat);
        assert_eq!(NorthSouth::North, parsed.north_south);
        assert_approx_eq!(0.07, parsed.lon);
        assert_eq!(EastWest::East, parsed.east_west);
        assert_approx_eq!(-47.7, parsed.alt);
        assert_eq!("W84".to_string(), parsed.ref_datum);
    }
}
