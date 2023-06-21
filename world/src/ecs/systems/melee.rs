use std::time::Duration;

use shipyard::{Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{guid::Guid, health::Health, melee::Melee, spell_cast::SpellCast, unit::Unit},
        resources::DeltaTime,
    },
    entities::position::WorldPosition,
    game::map::WrappedMap,
    protocol::{
        packets::{SmsgAttackStop, SmsgAttackerStateUpdate},
        server::ServerMessage,
    },
    shared::constants::{MeleeAttackError, WeaponAttackType, ATTACK_DISPLAY_DELAY},
};

pub fn attempt_melee_attack(
    _dt: UniqueView<DeltaTime>,
    map: UniqueView<WrappedMap>,
    v_guid: View<Guid>,
    mut v_health: ViewMut<Health>,
    mut v_melee: ViewMut<Melee>,
    v_unit: View<Unit>,
    v_wpos: View<WorldPosition>,
    v_spell: View<SpellCast>,
) {
    for (my_id, (guid, unit, my_position)) in (&v_guid, &v_unit, &v_wpos).iter().with_id() {
        if let Some(target_id) = unit.target() {
            let target_guid = v_guid
                .get(target_id)
                .expect("target has no Guid component")
                .0;
            let mut target_health = (&mut v_health)
                .get(target_id)
                .expect("target has no Health component");
            let target_position = v_wpos
                .get(target_id)
                .expect("target has no WorldPosition component");
            let target_melee_reach = {
                v_melee
                    .get(target_id)
                    .expect("target has no Melee component")
                    .melee_reach
            };

            let mut melee = (&mut v_melee).get(my_id).unwrap();
            let my_spell_cast = v_spell.get(my_id);

            if !melee.is_attacking || my_spell_cast.is_ok_and(|sp| sp.current_ranged().is_some()) {
                continue;
            }

            if !target_health.is_alive() {
                let packet = {
                    ServerMessage::new(SmsgAttackStop {
                        player_guid: guid.0.as_packed(),
                        enemy_guid: target_guid.as_packed(),
                        unk: 0,
                    })
                };

                map.0.broadcast_packet(&guid.0, &packet, None, true);

                melee.is_attacking = false;

                continue;
            }

            if !melee.can_reach_target_in_melee(my_position, target_position, target_melee_reach) {
                let my_session = map.0.get_session(&guid.0);
                melee.set_error(MeleeAttackError::NotInRange, my_session);

                melee.ensure_attack_time(WeaponAttackType::MainHand, Duration::from_millis(100));
                melee.ensure_attack_time(WeaponAttackType::OffHand, Duration::from_millis(100));
                continue;
            }

            if melee.is_attack_ready(WeaponAttackType::MainHand) {
                let damage = melee.damage();
                target_health.apply_damage(damage);

                let packet = ServerMessage::new(SmsgAttackerStateUpdate {
                    hit_info: 2, // TODO enum HitInfo
                    attacker_guid: guid.0.as_packed(),
                    target_guid: target_guid.as_packed(),
                    actual_damage: damage,
                    sub_damage_count: 1,
                    sub_damage_school_mask: 1, // Physical
                    sub_damage: 1.0,
                    sub_damage_rounded: damage,
                    sub_damage_absorb: 0,
                    sub_damage_resist: 0,
                    target_state: 1, // TODO: Enum VictimState
                    unk1: 0,
                    spell_id: 0,
                    damage_blocked_amount: 0,
                });

                // TODO: Replace all map_manager.0 with Unique<Map>
                map.0.broadcast_packet(&guid.0, &packet, None, true);

                melee.reset_attack_type(WeaponAttackType::MainHand);
                melee.ensure_attack_time(WeaponAttackType::OffHand, ATTACK_DISPLAY_DELAY);
                melee.set_error(MeleeAttackError::None, None);
            } else if melee.is_attack_ready(WeaponAttackType::OffHand) {
                todo!();
            }
        }
    }
}
