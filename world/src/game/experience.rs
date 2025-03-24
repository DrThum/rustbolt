use crate::{
    datastore::data_types::MapRecord,
    entities::{creature::Creature, player::Player},
    shared::constants::Expansion,
};

pub struct Experience;

impl Experience {
    pub fn xp_gain_against(player: &Player, creature: &Creature, map_record: &MapRecord) -> u32 {
        // TODO: 0 if creature is critter/totem/pet
        let player_level = player.level();
        let creature_level = creature.level_against(player_level);
        let base_xp = match map_record.expansion {
            Expansion::Vanilla => 45,
            Expansion::Tbc => 235,
        };

        let mut xp_gain = if creature_level >= player_level {
            let level_diff = (creature_level - player_level).min(4);
            ((player_level * 5 + base_xp) * (20 + level_diff) / 10 + 1) / 2
        } else {
            match Self::gray_level(player_level) {
                gray_level if creature_level > gray_level => {
                    let zd = Self::zero_difference(player_level);
                    (player_level * 5 + base_xp) * (zd + creature_level - player_level) / zd
                }
                _ => 0,
            }
        };

        if creature.template.rank.is_elite() {
            if map_record.is_dungeon() {
                xp_gain = (xp_gain as f32 * 2.75) as u32;
            } else {
                xp_gain = (xp_gain as f32 * 2.) as u32;
            }
        }

        xp_gain = (xp_gain as f32 * creature.template.experience_multiplier) as u32;

        xp_gain
    }

    /*
     * For a given character level, the amount of XP given by lower-level mobs is a linear function
     * of the Mob Level. The amount of experience reaches zero when the difference between the Char
     * Level and Mob Level reaches a certain point. This is called the Zero Difference value.
     */
    fn zero_difference(player_level: u32) -> u32 {
        match player_level {
            pl if pl < 8 => 5,
            pl if pl < 10 => 6,
            pl if pl < 12 => 7,
            pl if pl < 16 => 8,
            pl if pl < 20 => 9,
            pl if pl < 30 => 11,
            pl if pl < 40 => 12,
            pl if pl < 45 => 13,
            pl if pl < 50 => 14,
            pl if pl < 55 => 15,
            pl if pl < 60 => 16,
            _ => 17,
        }
    }

    fn gray_level(player_level: u32) -> u32 {
        match player_level {
            pl if pl <= 5 => 0,
            pl if pl <= 39 => pl - 5 - (pl / 10),
            pl if pl <= 59 => pl - 1 - (pl / 5),
            pl => pl - 9,
        }
    }
}
