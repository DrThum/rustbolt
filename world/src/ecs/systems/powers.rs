use shipyard::{Get, IntoIter, IntoWithId, View, ViewMut};

use crate::{
    ecs::components::{health::Powers, unit::Unit},
    entities::{creature::Creature, player::Player},
};

// Note: "Powers" includes health
pub fn regenerate_powers(
    mut vm_powers: ViewMut<Powers>,
    v_unit: View<Unit>,
    v_player: View<Player>,
    v_creature: View<Creature>,
) {
    for (entity_id, mut powers) in (&mut vm_powers).iter().with_id() {
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

        let can_regen_health =
            combat_state_allows_health_regen && powers.current_health() < powers.max_health();

        if can_regen_health {
            // Player case
            let health_to_regen = if let Ok(player) = v_player.get(entity_id) {
                player.health_regen_per_tick()
            } else if let Ok(_creature) = v_creature.get(entity_id) {
                // TODO: Creature case
                0.0
            } else {
                0.0
            };

            powers.apply_healing(health_to_regen as u32);
        }

        powers.reset_next_regen_time();
    }
}
