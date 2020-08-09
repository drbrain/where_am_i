use crate::nmea::*;

use serde::ser;
use serde::ser::Serialize;
use serde::ser::Serializer;
use serde::ser::SerializeStruct;

use std::fmt;

use std;
use std::fmt::Display;

pub type NResult<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Message(String),
    Eof,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::Eof => formatter.write_str("unexpected end of input"),
        }
    }
}

impl std::error::Error for Error {}

pub struct ToNMEA {
    output: String,
}

pub fn to_string<T>(value: &T) -> NResult<String>
where
    T: Serialize,
{
    let mut serializer = ToNMEA {
        output: String::new(),
    };

    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

impl Serialize for UBXPortMask {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
	S: Serializer,
    {
	let mut state = serializer.serialize_struct("UBXPortMask", 1)?;
	state.serialize_field("no comma", &self.bits())?;
	state.end()
    }
}

impl<'a> ser::Serializer for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> NResult<()> {
        if v {
            self.output += "1";
        } else {
            self.output += "0";
        }

        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> NResult<()> {
        Ok(())
    }

    fn serialize_char(self, v: char) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_f32(self, v: f32) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_f64(self, v: f64) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_i8(self, v: i8) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_i16(self, v: i16) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_i32(self, v: i32) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_i64(self, v: i64) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_map(self, _len: Option<usize>) -> NResult<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        v: &T,
    ) -> NResult<()>
    where
        T: ?Sized + Serialize,
    {
        v.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        v: &T,
    ) -> NResult<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += "{";
        variant.serialize(&mut *self)?;
        self.output += ":";
        v.serialize(&mut *self)?;
        self.output += "}";
        Ok(())
    }

    fn serialize_none(self) -> NResult<()> {
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> NResult<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_some<T>(self, v: &T) -> NResult<()>
    where
        T: ?Sized + Serialize, {
        v.serialize(self)
    }

    fn serialize_str(self, v: &str) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> NResult<Self::SerializeStruct> {
        match name {
            "UBXPositionPoll" => { self.output += "PUBX,00" },
            "UBXSvsPoll" => { self.output += "PUBX,03" },
            "UBXTimePoll" => { self.output += "PUBX,04" },
            "UBXRate" => { self.output += "PUBX,40" },
            "UBXConfig" => { self.output += "PUBX,41" },
            "UBXPortMask" => {},
            _ => panic!("don't know how to serialize struct {}", name),
        }

        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> NResult<Self::SerializeStructVariant> {
        eprintln!("serialize_struct_variant name: {}, variant: {}", _name, variant);
        variant.serialize(&mut *self)?;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> NResult<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> NResult<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> NResult<Self::SerializeTupleVariant> {
        variant.serialize(&mut *self)?;

        Ok(self)
    }

    fn serialize_u8(self, v: u8) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_u16(self, v: u16) -> NResult<()> {
        self.output += format!("{:04X}", v).as_str();

        Ok(())
    }

    fn serialize_u32(self, v: u32) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_u64(self, v: u64) -> NResult<()> {
        self.output += &v.to_string();

        Ok(())
    }

    fn serialize_unit(self) -> NResult<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> NResult<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> NResult<()> {
        self.serialize_str(variant)
    }
}

impl<'a> ser::SerializeMap for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, k: &T) -> NResult<()>
    where
        T: ?Sized + Serialize,
    {
	self.output += ",";

        k.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, v: &T) -> NResult<()>
    where
	T: ?Sized + Serialize,
    {
	self.output += ",";
	v.serialize(&mut **self)
    }

    fn end(self) -> NResult<()> {
	Ok(())
    }
}

impl<'a> ser::SerializeSeq for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, v: &T) -> NResult<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += ",";
        v.serialize(&mut **self)
    }

    fn end(self) -> NResult<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, k: &'static str, v: &T) -> NResult<()>
        where
            T: ?Sized + Serialize,
    {
        match k {
            "no comma" => (),
            _ => self.output += ",",
        }

        v.serialize(&mut **self)
    }

    fn end(self) -> NResult<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _k: &'static str, v: &T) -> NResult<()>
    where
	T: ?Sized + Serialize,
    {
        eprintln!("serialize_field key: {}", _k);
	self.output += ",";
	v.serialize(&mut **self)
    }

    fn end(self) -> NResult<()> {
	Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, v: &T) -> NResult<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += ",";

        v.serialize(&mut **self)
    }

    fn end(self) -> NResult<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, v: &T) -> NResult<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += ",";
        v.serialize(&mut **self)
    }

    fn end(self) -> NResult<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut ToNMEA {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, v: &T) -> NResult<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += ",";
        v.serialize(&mut **self)
    }

    fn end(self) -> NResult<()> {
        Ok(())
    }
}

