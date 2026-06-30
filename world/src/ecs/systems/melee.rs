use shipyard::{Get, IntoIter, IntoWithId, UniqueView, UniqueViewMut, View, ViewMut};

use crate::{
    ecs::{
        components::{
            guid::Guid,
            melee::{Melee, MeleeStrikeContext, MeleeStrikeOutcome},
            powers::Powers,
            spell_cast::SpellCast,
            threat_list::ThreatList,
            unit::Unit,
        },
        resources::CombatEvents,
        systems::combat::apply_combat_damage,
    },
    entities::{player::Player, position::WorldPosition},
    game::{map::HasPlayers, packet_broadcaster::WrappedPacketBroadcaster},
    protocol::{
        packets::{SmsgAttackStop, SmsgAttackerStateUpdate},
        server::ServerMessage,
    },
    session::session_holder::WrappedSessionHolder,
    shared::constants::MeleeAttackError,
};

// TODO: Move to systems/combat?
pub fn attempt_melee_attack(
    (has_players, packet_broadcaster, session_holder, mut combat_events): (
        UniqueView<HasPlayers>,
        UniqueView<WrappedPacketBroadcaster>,
        UniqueView<WrappedSessionHolder>,
        UniqueViewMut<CombatEvents>,
    ),
    v_guid: View<Guid>,
    v_unit: View<Unit>,
    mut vm_powers: ViewMut<Powers>,
    mut vm_melee: ViewMut<Melee>,
    mut vm_threat_list: ViewMut<ThreatList>,
    mut vm_player: ViewMut<Player>,
    v_wpos: View<WorldPosition>,
    v_spell: View<SpellCast>,
) {
    if !**has_players {
        return;
    }

    for (attacker_id, _) in (&mut vm_player).iter().with_id() {
        let attacker_position = v_wpos[attacker_id];
        let Some(target_id) = v_unit[attacker_id].target() else {
            continue;
        };

        let target_powers = vm_powers
            .get(target_id)
            .expect("target has no Health component");
        let target_position = v_wpos
            .get(target_id)
            .expect("target has no WorldPosition component");
        let target_melee_reach = {
            vm_melee
                .get(target_id)
                .expect("target has no Melee component")
                .melee_reach()
        };
        let attacker_guid = v_guid
            .get(attacker_id)
            .expect("attacker has no Guid component");
        let target_guid = v_guid.get(target_id).expect("target has no Guid component");

        let is_ranged_casting_in_progress = v_spell
            .get(attacker_id)
            .is_ok_and(|sp| sp.current_ranged().is_some());

        let context = MeleeStrikeContext {
            attacker_position,
            target_position: *target_position,
            target_melee_reach,
            is_target_alive: target_powers.is_alive(),
            is_ranged_casting_in_progress,
        };

        let mut melee = (&mut vm_melee)
            .get(attacker_id)
            .expect("attacker has no Melee component");

        let outcome = melee.resolve_strike(context);

        match outcome {
            MeleeStrikeOutcome::HitWithDamage { damage } => {
                apply_combat_damage(
                    attacker_id,
                    target_id,
                    damage,
                    &mut vm_powers,
                    &mut vm_threat_list,
                    &mut combat_events,
                );

                melee.set_error(MeleeAttackError::None, None);

                let packet = ServerMessage::new(SmsgAttackerStateUpdate {
                    hit_info: 2, // TODO enum HitInfo
                    attacker_guid: attacker_guid.as_packed(),
                    target_guid: target_guid.as_packed(),
                    actual_damage: damage as u32,
                    sub_damage_count: 1,
                    sub_damage_school_mask: 1, // Physical
                    sub_damage: 1.0,
                    sub_damage_rounded: damage as u32,
                    sub_damage_absorb: 0,
                    sub_damage_resist: 0,
                    target_state: 1, // TODO: Enum VictimState
                    unk1: 0,
                    spell_id: 0,
                    damage_blocked_amount: 0,
                });

                packet_broadcaster.broadcast_packet(&attacker_guid, &packet, None, true);
            }
            MeleeStrikeOutcome::TargetDead => {
                let packet = {
                    ServerMessage::new(SmsgAttackStop {
                        attacker_guid: attacker_guid.as_packed(),
                        enemy_guid: target_guid.as_packed(),
                        unk: 0,
                    })
                };

                packet_broadcaster.broadcast_packet(&attacker_guid, &packet, None, true);

                melee.is_attacking = false;
            }
            MeleeStrikeOutcome::OutOfRange => {
                let my_session = session_holder.get_session(&attacker_guid);
                melee.set_error(MeleeAttackError::NotInRange, my_session);
            }
            MeleeStrikeOutcome::NotAttacking => (),
        };
    }
}
