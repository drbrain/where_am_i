mod codec;
mod parser;

pub use codec::Codec;
pub use parser::NMEA;

#[cfg(test)]
mod test;
