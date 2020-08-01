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

fn any<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, String, E> {
    let (input, matched) = take_while1(|c| c != ',')(input)?;

    Ok((input, matched.to_string()))
}

fn checksum<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u32, E> {
    let (input, checksum) = preceded(char('*'), hex_digit1)(input)?;

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

fn east_west<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, char, E> {
    alt((char('E'), char('W')))(input)
}

fn north_south<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, char, E> {
    alt((char('N'), char('S')))(input)
}

fn talker<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Talker, E> {
    let (input, talker) =
        alt((tag("GA"), tag("GB"), tag("GL"), tag("GN"), tag("GP")))(input)?;

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

#[derive(Clone, Debug, PartialEq)]
pub struct DTMDatum {
    pub talker: Talker,
    pub datum: String,
    pub subDatum: String,
    pub lat: f32,
    pub lon: f32,
    pub alt: f32,
    pub refDatum: String,
}

fn dtm<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, DTMDatum, E> {
    let (input, (talker, _, _, datum, _, subDatum, _, lat, _, north_south, _, lon, _, east_west, _,
            alt, _, refDatum, checksum)) =
    preceded(dollar, terminated(tuple((talker, tag("DTM"), comma, any, comma, any, comma,
        recognize_float, comma, north_south, comma, recognize_float, comma, east_west, comma,
        recognize_float, comma, any, checksum)), eol))(input)?;

    let lat = lat.parse().unwrap();
    let lon = lon.parse().unwrap();
    let alt = alt.parse().unwrap();

    let dtm_datum = DTMDatum {
        talker,
        datum,
        subDatum,
        lat,
        lon,
        alt,
        refDatum,
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
    fn test_talker() {
        assert_eq!(Talker::Galileo, talker::<()>("GA").unwrap().1);
        assert_eq!(Talker::BeiDuo, talker::<()>("GB").unwrap().1);
        assert_eq!(Talker::GLONASS, talker::<()>("GL").unwrap().1);
        assert_eq!(Talker::Combination, talker::<()>("GN").unwrap().1);
        assert_eq!(Talker::GPS, talker::<()>("GP").unwrap().1);
    }

    fn test_DTM() {
        let parsed = dtm::<()>("$GPDTM,W84,,0.0,N,0.0,E,0.0,W84*6F").unwrap().1;

        assert_eq!(Talker::GPS, parsed.talker);
        assert_eq!("W84".to_string(), parsed.datum);
        assert_eq!("".to_string(), parsed.subDatum);
        assert_approx_eq!(0.8, parsed.lat);
        assert_approx_eq!(0.07, parsed.lon);
        assert_approx_eq!(-47.7, parsed.alt);
        assert_eq!("W84".to_string(), parsed.refDatum);
    }
}
