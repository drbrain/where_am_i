mod codec;
pub mod device;
pub mod parser;
pub mod parser_util;
mod sentence_parser;
mod ser;

pub use codec::Codec;

pub use device::Device;
pub use device::SerialCodec;

pub use parser::NavigationMode;
pub use parser::OperationMode;
pub use parser::Quality;
pub use parser::System;
pub use parser::Talker;
pub use parser::NMEA;

pub use parser_util::EastWest;
pub use parser_util::LatLon;
pub use parser_util::NorthSouth;

pub use parser::DTMData;
pub use parser::GAQData;
pub use parser::GBQData;
pub use parser::GBSData;
pub use parser::GGAData;
pub use parser::GLLData;
pub use parser::GLQData;
pub use parser::GNQData;
pub use parser::GNSData;
pub use parser::GPQData;
pub use parser::GRSData;
pub use parser::GSAData;
pub use parser::GSTData;
pub use parser::GSVData;
pub use parser::RMCData;
pub use parser::TXTData;
pub use parser::VLWData;
pub use parser::VTGData;
pub use parser::ZDAData;

pub use parser::message;

pub use ser::ToNMEA;

#[cfg(test)]
mod test_codec;

#[cfg(test)]
mod test_parser;

#[cfg(test)]
mod test_sentence_parser;

#[cfg(test)]
mod test_ser;
