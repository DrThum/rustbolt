use std::fmt;

use binrw::{binread, io::Cursor, BinReaderExt};
use downcast_rs::{impl_downcast, Downcast};

pub mod wdt;

#[binread]
#[derive(Debug)]
pub struct FileChunk {
    magic: [u8; 4],
    size: u32,
    #[br(count = size)]
    data: Vec<u8>,
}

impl FileChunk {
    pub fn as_typed(&self) -> Box<dyn TypedFileChunk> {
        match self.magic {
            [b'R', b'E', b'V', b'M'] => Box::new(MVER::parse(&self.data).unwrap()),
            [b'D', b'H', b'P', b'M'] => Box::new(wdt::MPHD::parse(&self.data).unwrap()),
            [b'N', b'I', b'A', b'M'] => Box::new(wdt::MAIN::parse(&self.data).unwrap()),
            [b'O', b'M', b'W', b'M'] => Box::new(wdt::MWMO::parse(&self.data).unwrap()),
            _ => {
                panic!(
                    "Unsupported chunk {:?} of size {}",
                    String::from_utf8(self.magic.to_vec()).unwrap(),
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
}

pub trait TypedFileChunk: Downcast {
    fn name(&self) -> &str;
    fn content_as_string(&self) -> String;
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
