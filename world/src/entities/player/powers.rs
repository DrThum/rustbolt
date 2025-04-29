use std::time::Instant;

use crate::{datastore::data_types::GAME_TABLE_MAX_LEVEL, shared::constants::UnitAttribute};

use super::{Player, UnitFields};

impl Player {
    pub fn health_regen_per_tick(&self) -> f32 {
        let level = self.level().min(GAME_TABLE_MAX_LEVEL);
        let class = self
            .internal_values
            .read()
            .get_u8(UnitFields::UnitFieldBytes0.into(), 0) as u32;
        let spirit_stat = self
            .internal_values
            .read()
            .get_u32(UnitFields::UnitFieldStat0 as usize + UnitAttribute::Spirit as usize)
            as f32;
        let base_spirit = spirit_stat.min(50.0);
        let extra_spirit = spirit_stat - base_spirit;

        let index = ((class - 1) * GAME_TABLE_MAX_LEVEL + level - 1) as usize;
        let maybe_base_regen_hp_record = self.world_context.data_store.get_gtOCTRegenHP(index);
        let maybe_regen_hp_from_spirit_record =
            self.world_context.data_store.get_gtRegenHPPerSpt(index);

        match (
            maybe_base_regen_hp_record,
            maybe_regen_hp_from_spirit_record,
        ) {
            (None, _) | (_, None) => 0.0,
            (Some(base_record), Some(from_spirit_record)) => {
                base_spirit * base_record.ratio + extra_spirit * from_spirit_record.ratio
            }
        }
    }

    pub fn mana_regen_per_tick(&self) -> f32 {
        let has_cast_recently = Instant::now() >= self.partial_regen_period_end;
        let values_index = if has_cast_recently {
            UnitFields::PlayerFieldModManaRegenInterrupt
        } else {
            UnitFields::PlayerFieldModManaRegen
        };
        let mana_regen = self.internal_values.read().get_f32(values_index as usize);

        mana_regen * 2.
    }

    pub fn energy_regen_per_tick(&self) -> f32 {
        // TODO: Use SPELL_AURA_MOD_POWER_REGEN_PERCENT
        20.
    }

    pub fn rage_degen_per_tick(&self) -> f32 {
        // TODO: Use SPELL_AURA_MOD_POWER_REGEN_PERCENT
        20. // Note: this probably needs to be multiplied by 10
    }
}
