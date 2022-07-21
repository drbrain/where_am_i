#[cfg(test)]
mod test {
    use crate::gps::mkt::*;

    #[test]
    fn test_mkt_010() {
        let parsed = mkt_010("PMTK010,000").unwrap().1;

        assert_eq!(MKTSystemMessage::Unknown, parsed);

        let parsed = mkt_010("PMTK010,001").unwrap().1;

        assert_eq!(MKTSystemMessage::Startup, parsed);

        let parsed = mkt_010("PMTK010,004").unwrap().1;

        assert_eq!(MKTSystemMessage::Unhandled(4), parsed);
    }

    #[test]
    fn test_mkt_011() {
        let parsed = mkt_011("PMTK011,MTKGPS").unwrap().1;

        assert_eq!("MTKGPS", parsed.message);
    }
}
