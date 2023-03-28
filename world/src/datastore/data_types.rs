#[derive(Debug)]
pub struct ChrRacesRecord {
    // _flags: u32,
    pub _faction_id: u32,
    // _exploration_sound_id: u32,
    pub male_display_id: u32,
    pub female_display_id: u32,
    // _client_prefix: String, // stringref (offset into the String block of the DBC file)
    // _mount_scale: f32,
    // _base_language: u32,         // 1 = Horde, 7 = Alliance & Not Playable
    // _creature_type: u32,         // Always 7 (humanoid)
    pub _res_sickness_spell_id: u32, // Always 15007
    // _splash_sound_id: u32,
    // _client_file_string: String,
    // _opening_cinematic_id: u32, // Ref to another DBC
    // _race_name_neutral: LocalizedString,
    // _race_name_female: LocalizedString,
    // _race_name_male: LocalizedString,
    // _facial_hair_customization_internal: String,
    // _facial_hair_customization_lua: String,
    // _hair_customization: String,
    pub _required_expansion: u32,
}
