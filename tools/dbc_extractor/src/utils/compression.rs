use bzip2::{Decompress, Error, Status};

#[allow(dead_code)]
pub enum CompressionTypeFlags {
    Huffman = 0x01,
    Zlib = 0x02,
    PkZip = 0x08, // pkware dcl compression
    Bzip2 = 0x10,
    WaveMono = 0x40,
    WaveStereo = 0x80,
}

// http://web.archive.org/web/20090521194815/http://wiki.devklog.net/index.php?title=The_MoPaQ_Archive_Format

pub fn decompress_bzip2(input: Vec<u8>, output: &mut Vec<u8>) -> Result<Status, Error> {
    let mut decompressor = Decompress::new(false);

    decompressor.decompress_vec(&input, output)
}

pub fn decompress_zlib(input: Vec<u8>) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec_zlib(&input).unwrap()
}

pub fn decompress(input: Vec<u8>, compression_flags: u8) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();

    if compression_flags & (CompressionTypeFlags::Bzip2 as u8) != 0 {
        decompress_bzip2(input, &mut output).unwrap();
    } else {
        output = input;
    }

    if compression_flags & (CompressionTypeFlags::PkZip as u8) != 0 {
        panic!("PkZip DCL decompression not implemented yet");
    }

    if compression_flags & (CompressionTypeFlags::Zlib as u8) != 0 {
        output = decompress_zlib(output);
    }

    if compression_flags & (CompressionTypeFlags::Huffman as u8) != 0 {
        panic!("Huffman decompression not implemented yet");
    }

    if compression_flags & (CompressionTypeFlags::WaveStereo as u8) != 0 {
        panic!("WaveStereo decompression not implemented yet");
    }

    if compression_flags & (CompressionTypeFlags::WaveMono as u8) != 0 {
        panic!("WaveMono decompression not implemented yet");
    }

    output
}

// https://docs.rs/implode/latest/implode/
