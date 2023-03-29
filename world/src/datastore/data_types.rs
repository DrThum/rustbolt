use super::dbc::DbcRecord;

pub trait DbcTypedRecord {
    fn from_record(record: &DbcRecord) -> (u32, Self);
}

#[derive(Debug)]
pub struct ChrRacesRecord {
    // _flags: u32,
    // _faction_id: u32,
    // _exploration_sound_id: u32,
    pub male_display_id: u32,
    pub female_display_id: u32,
    // _client_prefix: String, // stringref (offset into the String block of the DBC file)
    // _mount_scale: f32,
    // _base_language: u32,         // 1 = Horde, 7 = Alliance & Not Playable
    // _creature_type: u32,         // Always 7 (humanoid)
    // _res_sickness_spell_id: u32, // Always 15007
    // _splash_sound_id: u32,
    // _client_file_string: String,
    // _opening_cinematic_id: u32, // Ref to another DBC
    // _race_name_neutral: LocalizedString,
    // _race_name_female: LocalizedString,
    // _race_name_male: LocalizedString,
    // _facial_hair_customization_internal: String,
    // _facial_hair_customization_lua: String,
    // _hair_customization: String,
    // _required_expansion: i32,
}

impl DbcTypedRecord for ChrRacesRecord {
    fn from_record(record: &DbcRecord) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = ChrRacesRecord {
                male_display_id: record.fields[4].as_u32,
                female_display_id: record.fields[5].as_u32,
            };

            (key, record)
        }
    }
}

pub struct ChrClassesRecord {
    // _unk: u32,
    pub power_type: u32, // See enum PowerType
                         // _pet_name_token: String, // string ref
                         // _name: LocalizedString,
                         // _name_female: LocalizedString,
                         // _name_male: LocalizedString,
                         // _file_name: String,
                         // _spell_class_set: u32, // https://wowdev.wiki/DB/ChrClasses#spellClassSet
                         // _flags: u32, // https://wowdev.wiki/DB/ChrClasses#Flags
}

impl DbcTypedRecord for ChrClassesRecord {
    fn from_record(record: &DbcRecord) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = ChrClassesRecord {
                power_type: record.fields[2].as_u32,
            };

            (key, record)
        }
    }
}
