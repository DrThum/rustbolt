use binrw::{io::Cursor, BinReaderExt};

use super::TypedFileChunk;

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
