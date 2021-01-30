#[cfg(test)]
mod test {
    use crate::gps::mkt::*;

    use nom::error::*;

    type VE<'a> = VerboseError<&'a str>;

    #[test]
    fn test_mkt_010() {
        let parsed = mkt_010::<VE>("PMTK010,000").unwrap().1;

        assert_eq!(MKTSystemMessage::Unknown, parsed);

        let parsed = mkt_010::<VE>("PMTK010,001").unwrap().1;

        assert_eq!(MKTSystemMessage::Startup, parsed);

        let parsed = mkt_010::<VE>("PMTK010,004").unwrap().1;

        assert_eq!(MKTSystemMessage::Unhandled(4), parsed);
    }

    #[test]
    fn test_mkt_011() {
        let parsed = mkt_011::<VE>("PMTK011,MTKGPS").unwrap().1;

        assert_eq!("MTKGPS", parsed.message);
    }
}
