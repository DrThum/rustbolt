use std::sync::Arc;

use crate::{
    datastore::data_types::{SkillLineAbilityRecord, SkillRaceClassInfoRecord},
    game::world_context::WorldContext,
    shared::constants::AbilitySkillFlags,
};

use super::Player;

impl Player {
    /**
     * Checks whether the player has the required race and class for a spell that has an associated skill line.
     * DOES NOT check anything regarding the level, so "true" is more like "the player will be able the train the spell eventually".
     */
    pub fn can_train_spell(&self, spell_id: u32, world_context: Arc<WorldContext>) -> bool {
        fn is_skill_line_ability_allowed(
            player: &Player,
            skill_line_ability: &SkillLineAbilityRecord,
            world_context: Arc<WorldContext>,
        ) -> bool {
            let race_ok = skill_line_ability.race_mask.is_empty()
                || skill_line_ability.race_mask.intersects(player.race_bit());
            let class_ok = skill_line_ability.class_mask.is_empty()
                || skill_line_ability.class_mask.intersects(player.class_bit());

            let skill_race_class_infos = world_context
                .data_store
                .get_skill_race_class_info_by_skill_id(skill_line_ability.skill_id as u32);

            let skill_race_class_ok = match skill_race_class_infos {
                None => true,
                Some(skill_race_class_infos) => skill_race_class_infos
                    .iter()
                    .all(|info| is_skill_race_class_info_allowed(player, info)),
            };

            race_ok && class_ok && skill_race_class_ok
        }

        fn is_skill_race_class_info_allowed(
            player: &Player,
            skill_race_class_info: &SkillRaceClassInfoRecord,
        ) -> bool {
            if !skill_race_class_info
                .race_mask
                .intersects(player.race_bit())
            {
                return true;
            }

            if !skill_race_class_info
                .class_mask
                .intersects(player.class_bit())
            {
                return true;
            }

            if skill_race_class_info.flags & (AbilitySkillFlags::NonTrainable as u32) != 0 {
                return false;
            }

            player.level() >= skill_race_class_info.required_level
        }

        let Some(skill_line_abilities) = world_context
            .data_store
            .get_skill_line_ability_by_spell(spell_id)
        else {
            return true; // The spell has no associated skill line
        };

        skill_line_abilities.iter().all(|skill_line_ability| {
            is_skill_line_ability_allowed(self, skill_line_ability, world_context.clone())
        })
    }
}
