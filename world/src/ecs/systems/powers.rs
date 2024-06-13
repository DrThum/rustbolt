use shipyard::{Get, IntoIter, IntoWithId, View, ViewMut};

use crate::{
    ecs::components::{powers::Powers, unit::Unit},
    entities::{creature::Creature, player::Player},
    shared::constants::PowerType,
};

// Note: "Powers" includes health
pub fn regenerate_powers(
    mut vm_powers: ViewMut<Powers>,
    v_unit: View<Unit>,
    v_player: View<Player>,
    v_creature: View<Creature>,
) {
    for (entity_id, powers) in (&mut vm_powers).iter().with_id() {
        if !powers.is_past_next_regen_time() {
            continue;
        }

        // Regen health if not in combat (or if no combat component)
        // TODO: SPELL_AURA_MOD_REGEN_DURING_COMBAT or SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT or
        // Polymorph allow to regen health in combat
        // Creatures have special rules too
        let combat_state_allows_health_regen = v_unit
            .get(entity_id)
            .map(|unit| !unit.combat_state())
            .unwrap_or(true);

        let maybe_player = v_player.get(entity_id);
        let maybe_creature = v_creature.get(entity_id);

        let can_regen_health =
            combat_state_allows_health_regen && powers.current_health() < powers.max_health();

        if can_regen_health {
            // Player case
            let health_to_regen = if let Ok(player) = maybe_player {
                player.health_regen_per_tick()
            } else if let Ok(_creature) = maybe_creature {
                // TODO: Creature case
                0.0
            } else {
                0.0
            };

            powers.apply_healing(health_to_regen as u32);
        }

        // Regen mana
        let mana_to_regen = if let Ok(player) = maybe_player {
            player.mana_regen_per_tick()
        } else if let Ok(_creature) = maybe_creature {
            // TODO: Creature case
            0.0
        } else {
            0.0
        };

        powers.modify_power(&PowerType::Mana, mana_to_regen as i32);

        // Regen energy
        let energy_to_regen = if let Ok(player) = maybe_player {
            player.energy_regen_per_tick()
        } else if let Ok(_creature) = maybe_creature {
            // TODO: Creature case
            0.0
        } else {
            0.0
        };

        powers.modify_power(&PowerType::Energy, energy_to_regen as i32);

        // "Regen" rage
        let rage_to_degen = if let Ok(player) = maybe_player {
            player.rage_degen_per_tick()
        } else if let Ok(_creature) = maybe_creature {
            // TODO: Creature case
            0.0
        } else {
            0.0
        };

        powers.modify_power(&PowerType::Rage, rage_to_degen as i32 * -1);

        powers.reset_next_regen_time();
    }
}
