use std::fmt;

use binrw::{binread, io::Cursor, BinReaderExt};
use downcast_rs::{impl_downcast, Downcast};

pub mod adt;
pub mod wdt;
pub mod wmo;

#[derive(PartialEq, Debug)]
pub enum FileType {
    WDT,
    ADT,
    WMO,
}

#[binread]
#[derive(Debug)]
pub struct FileChunk {
    magic: [u8; 4],
    size: u32,
    #[br(count = size)]
    data: Vec<u8>,
}

impl FileChunk {
    pub fn as_typed(&self, file_type: FileType) -> Box<dyn TypedFileChunk> {
        match self.magic_str().as_str() {
            "MVER" => Box::new(MVER::parse(&self.data).unwrap()),
            "MPHD" => Box::new(wdt::MPHD::parse(&self.data).unwrap()),
            "MAIN" => Box::new(wdt::MAIN::parse(&self.data).unwrap()),
            "MWMO" if file_type == FileType::WDT => Box::new(wdt::MWMO::parse(&self.data).unwrap()),
            "MWMO" if file_type == FileType::ADT => Box::new(adt::MWMO::parse(&self.data).unwrap()),
            "MHDR" => Box::new(adt::MHDR::parse(&self.data).unwrap()),
            "MCIN" => Box::new(adt::MCIN::parse(&self.data).unwrap()),
            "MTEX" => Box::new(adt::MTEX::parse(&self.data).unwrap()),
            "MMDX" => Box::new(adt::MMDX::parse(&self.data).unwrap()),
            "MMID" => Box::new(adt::MMID::parse(&self.data).unwrap()),
            "MWID" => Box::new(adt::MWID::parse(&self.data).unwrap()),
            "MDDF" => Box::new(adt::MDDF::parse(&self.data).unwrap()),
            "MODF" => Box::new(adt::MODF::parse(&self.data).unwrap()),
            "MCNK" => Box::new(adt::MCNK::parse(&self.data).unwrap()),
            "MOHD" => Box::new(wmo::MOHD::parse(&self.data).unwrap()),
            "MOGP" => Box::new(wmo::MOGP::parse(&self.data).unwrap()),
            "MOPY" => Box::new(wmo::MOPY::parse(&self.data).unwrap()),
            "MOVI" => Box::new(wmo::MOVI::parse(&self.data).unwrap()),
            "MOVT" => Box::new(wmo::MOVT::parse(&self.data).unwrap()),
            "MONR" => Box::new(wmo::MONR::parse(&self.data).unwrap()),
            "MOTV" => Box::new(wmo::MOTV::parse(&self.data).unwrap()),
            "MOBA" => Box::new(wmo::MOBA::parse(&self.data).unwrap()),
            "MOBN" => Box::new(wmo::MOBN::parse(&self.data).unwrap()),
            "MOBR" => Box::new(wmo::MOBR::parse(&self.data).unwrap()),
            "MLIQ" => Box::new(wmo::MLIQ::parse(&self.data).unwrap()),
            _ => {
                panic!(
                    "Unsupported chunk {:?} of size {}",
                    self.magic_str(),
                    self.size
                );
            }
        }
    }

    pub fn magic_str(&self) -> String {
        let mut magic = self.magic.to_vec();
        magic.reverse();
        String::from_utf8(magic).unwrap()
    }

    pub fn read_as<T: TypedFileChunk>(
        file_type: FileType,
        reader: &mut Cursor<&Vec<u8>>,
    ) -> Box<T> {
        let chunk: FileChunk = reader
            .read_le()
            .expect(&format!("failed to read chunk from {:?} file", file_type));

        chunk
            .as_typed(file_type)
            .downcast::<T>()
            .unwrap_or_else(|_| {
                panic!("got unexpected chunk {}", chunk.magic_str());
            })
    }
}

pub trait TypedFileChunk: Downcast {
    fn name(&self) -> &str;
    fn content_as_string(&self) -> String {
        "".to_string()
    }
}
impl_downcast!(TypedFileChunk);

impl fmt::Debug for dyn TypedFileChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Chunk\n\t{}", self.name(), self.content_as_string())
    }
}

pub struct MVER {
    version: u32,
}

impl MVER {
    pub fn parse(raw: &Vec<u8>) -> Result<MVER, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let version: u32 = reader.read_le()?;

        Ok(MVER { version })
    }
}

impl TypedFileChunk for MVER {
    fn name(&self) -> &str {
        "MVER"
    }

    fn content_as_string(&self) -> String {
        format!("version {}", self.version)
    }
}
