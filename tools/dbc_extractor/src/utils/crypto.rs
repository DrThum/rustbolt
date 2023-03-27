// http://www.zezula.net/en/mpq/techinfo.html#Encryption
pub fn prepare_crypt_table(crypt_table: &mut [u32; 0x500]) {
    let mut seed: u32 = 0x00100001;

    for index1 in 0..0x100 {
        let mut index2 = index1;

        for _i in 0..5 {
            seed = (seed * 125 + 3) % 0x2AAAAB;
            let temp1: u32 = (seed & 0xFFFF) << 0x10;

            seed = (seed * 125 + 3) % 0x2AAAAB;
            let temp2: u32 = seed & 0xFFFF;

            crypt_table[index2] = temp1 | temp2;

            index2 += 0x100;
        }
    }
}

pub fn decrypt_block_in_place(data: &mut Vec<u32>, key: u32, crypt_table: &[u32; 0x500]) {
    let mut key = key;
    let mut seed: u32 = 0xEEEEEEEE;
    for idx in 0..data.len() {
        let crypt_table_index: usize = (0x400 + (key & 0xFF)).try_into().unwrap();
        seed = seed.wrapping_add(crypt_table[crypt_table_index]);

        let ch = data[idx] ^ (key.wrapping_add(seed));
        key = ((!key << 0x15) + 0x11111111) | (key >> 0x0B);
        seed = ch.wrapping_add(seed.wrapping_add(seed << 5).wrapping_add(3));

        data[idx] = ch;
    }
}
