use binrw::binread;

#[binread]
struct ClientMessageHeader {
    #[br(big)]
    pub size: u16,
    pub opcode: u32,
}
