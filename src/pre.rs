use std::io::BufWriter;

pub use anyhow::{format_err, Error, Result};
pub use aamp::ParameterIO;
pub use byml::Byml;
pub use msyt::Msyt;
pub use sarc_rs::{Sarc, SarcWriter};
pub use yaz0::{Yaz0Archive, Yaz0Writer};

pub const COMPRESS: yaz0::CompressionLevel = yaz0::CompressionLevel::Lookahead { quality: 6 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Endian {
    Big,
    Little,
}

impl From<byml::Endian> for Endian {
    fn from(endian: byml::Endian) -> Self {
        match endian {
            byml::Endian::Big => Endian::Big,
            byml::Endian::Little => Endian::Little,
        }
    }
}

impl Into<byml::Endian> for Endian {
    fn into(self) -> byml::Endian {
        match self {
            Self::Big => byml::Endian::Big,
            Self::Little => byml::Endian::Little,
        }
    }
}

impl From<sarc_rs::Endian> for Endian {
    fn from(endian: sarc_rs::Endian) -> Self {
        match endian {
            sarc_rs::Endian::Big => Endian::Big,
            sarc_rs::Endian::Little => Endian::Little,
        }
    }
}

impl Into<sarc_rs::Endian> for Endian {
    fn into(self) -> sarc_rs::Endian {
        match self {
            Self::Big => sarc_rs::Endian::Big,
            Self::Little => sarc_rs::Endian::Little,
        }
    }
}

impl From<msyt::Endianness> for Endian {
    fn from(endianness: msyt::Endianness) -> Self {
        match endianness {
            msyt::Endianness::Big => Self::Big,
            msyt::Endianness::Little => Self::Little,
        }
    }
}

impl Into<msyt::Endianness> for Endian {
    fn into(self) -> msyt::Endianness {
        match self {
            Self::Big => msyt::Endianness::Big,
            Self::Little => msyt::Endianness::Little,
        }
    }
}

pub(crate) fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut buffer: Vec<u8> = Vec::with_capacity(data.len());
    Yaz0Writer::new(&mut BufWriter::new(&mut buffer))
        .compress_and_write(&data, COMPRESS)?;
    Ok(buffer)
}
