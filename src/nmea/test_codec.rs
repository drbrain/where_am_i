#[cfg(test)]
mod test {
    use crate::nmea::Codec;
    use crate::nmea::NMEA;

    use bytes::BytesMut;

    use tokio_util::codec::Decoder;

    #[test]
    fn test_nmea_codec_same_read() {
        let mut codec = Codec::default();

        // these two lines appeared in a single read from the GPS device
        let mut bytes_mut = BytesMut::new();
        bytes_mut.extend_from_slice(
            b"$GPGGA,025134.000,4735.2887,N,12217.9631,W,1,10,0.90,27.1,M,-17.3,M,,*61\r\n",
        );
        bytes_mut.extend_from_slice(b"$GPGLL,4735.2887,N,12217.9631,W,025134.000,A,A*40\r\n");

        let first = match codec.decode(&mut bytes_mut).unwrap().unwrap() {
            NMEA::GGA(gga) => gga.received,
            _ => unreachable!("first message is GGA"),
        };

        let second = match codec.decode(&mut bytes_mut).unwrap().unwrap() {
            NMEA::GLL(gll) => gll.received,
            _ => unreachable!("second message is GLL"),
        };

        assert_ne!(first, second);
    }

    #[test]
    fn test_nmea_codec_different_read() {
        let mut codec = Codec::default();

        // The second line appeared in a subsequent read
        let mut bytes_mut = BytesMut::new();
        bytes_mut.extend_from_slice(
            b"$GPGGA,025134.000,4735.2887,N,12217.9631,W,1,10,0.90,27.1,M,-17.3,M,,*61\r\n",
        );

        let first = match codec.decode(&mut bytes_mut).unwrap().unwrap() {
            NMEA::GGA(gga) => gga.received,
            _ => unreachable!("first message is GGA"),
        };

        bytes_mut.extend_from_slice(b"$GPGLL,4735.2887,N,12217.9631,W,025134.000,A,A*40\r\n");

        let second = match codec.decode(&mut bytes_mut).unwrap().unwrap() {
            NMEA::GLL(gll) => gll.received,
            _ => unreachable!("second message is GLL"),
        };

        assert_ne!(first, second);
    }
}
