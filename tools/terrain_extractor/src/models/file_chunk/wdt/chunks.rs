use binrw::{binread, io::Cursor, BinReaderExt, NullString};

use crate::models::file_chunk::TypedFileChunk;

pub struct MPHD {
    flags: u32,
    _unk: u32,
    _unused: Vec<u32>,
}

impl MPHD {
    pub fn parse(raw: &Vec<u8>) -> Result<MPHD, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let data: [u32; 8] = reader.read_le()?;

        Ok(MPHD {
            flags: data[0],
            _unk: data[1],
            _unused: data[..2].to_vec(),
        })
    }
}

impl TypedFileChunk for MPHD {
    fn name(&self) -> &str {
        "MPHD"
    }

    fn content_as_string(&self) -> String {
        format!("flags {:X}", self.flags)
    }
}

#[binread]
#[derive(Debug)]
pub struct SMAreaInfo {
    #[br(map = |x: u32| if x == 1 { true } else { false })]
    pub exists: bool,
    _data1: u32, // Always 0
}

pub struct MAIN {
    pub areas: [SMAreaInfo; 64 * 64],
}

impl MAIN {
    pub fn parse(raw: &Vec<u8>) -> Result<MAIN, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let areas: [SMAreaInfo; 64 * 64] = reader.read_le()?;

        Ok(MAIN { areas })
    }
}

impl TypedFileChunk for MAIN {
    fn name(&self) -> &str {
        "MAIN"
    }

    fn content_as_string(&self) -> String {
        let num_existing = self.areas.iter().filter(|a| a.exists).count();
        format!("number of areas: {}", num_existing)
    }
}

pub struct MWMO {
    wmo_name: Option<NullString>,
}

impl MWMO {
    pub fn parse(raw: &Vec<u8>) -> Result<MWMO, binrw::Error> {
        let mut wmo_name: Option<NullString> = None;

        if !raw.is_empty() {
            let mut reader = Cursor::new(raw);
            let name: NullString = reader.read_le()?;
            wmo_name = Some(name);
        }

        Ok(MWMO { wmo_name })
    }
}

impl TypedFileChunk for MWMO {
    fn name(&self) -> &str {
        "MWMO"
    }

    fn content_as_string(&self) -> String {
        format!("wmo: {:?}", self.wmo_name)
    }
}
