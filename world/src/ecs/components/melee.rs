use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use log::{error, warn};
use parking_lot::RwLock;
use shared::utils::value_range::ValueRange;
use shipyard::{Component, EntityId, Get, View, ViewMut};

use crate::{
    entities::{
        creature::Creature, internal_values::InternalValues, player::Player,
        position::WorldPosition, update_fields::UnitFields,
    },
    game::{experience::Experience, map::Map},
    protocol::{
        packets::{SmsgAttackStop, SmsgAttackSwingNotInRange, SmsgAttackerStateUpdate},
        server::ServerMessage,
    },
    session::world_session::WorldSession,
    shared::constants::{
        MeleeAttackError, SheathState, UnitDynamicFlag, WeaponAttackType, ATTACK_DISPLAY_DELAY,
        BASE_MELEE_RANGE_OFFSET, NUMBER_WEAPON_ATTACK_TYPES,
    },
    DataStore,
};

use super::{
    guid::Guid, powers::Powers, spell_cast::SpellCast, threat_list::ThreatList, unit::Unit,
};

#[derive(Component)]
pub struct Melee {
    internal_values: Arc<RwLock<InternalValues>>,
    damage_interval: ValueRange<f32>,
    pub is_attacking: bool,
    next_attack_times: [Instant; NUMBER_WEAPON_ATTACK_TYPES], // MainHand, OffHand, Ranged
    attack_intervals: [Duration; NUMBER_WEAPON_ATTACK_TYPES],
    has_off_hand: bool,
    last_error: MeleeAttackError,
    sheath_state: SheathState,
}

impl Melee {
    pub fn new(
        internal_values: Arc<RwLock<InternalValues>>,
        damage_min: f32,
        damage_max: f32,
        is_default_attacking: bool,
        attack_intervals: [Duration; 3],
    ) -> Self {
        let now = Instant::now();

        {
            let mut guard = internal_values.write();
            guard.set_u8(
                UnitFields::UnitFieldBytes2.into(),
                0,
                SheathState::Unarmed as u8,
            );

            // TODO: multiply these by modifiers
            guard.set_u32(
                UnitFields::UnitFieldBaseAttackTime.into(),
                attack_intervals[0].as_millis() as u32,
            );
            guard.set_u32(
                UnitFields::UnitFieldBaseAttackTime as usize + 1,
                attack_intervals[1].as_millis() as u32,
            );
            guard.set_u32(
                UnitFields::UnitFieldRangedAttackTime.into(),
                attack_intervals[2].as_millis() as u32,
            );
            guard.set_f32(UnitFields::UnitFieldMinDamage.into(), damage_min);
            guard.set_f32(UnitFields::UnitFieldMaxDamage.into(), damage_max);
        }

        Self {
            internal_values,
            damage_interval: ValueRange::new(damage_min, damage_max),
            is_attacking: is_default_attacking,
            next_attack_times: [now, now, now],
            attack_intervals,
            has_off_hand: false,
            last_error: MeleeAttackError::None,
            sheath_state: SheathState::Unarmed,
        }
    }

    pub fn calc_damage(&self) -> f32 {
        self.damage_interval.random_value()
    }

    pub fn melee_reach(&self) -> f32 {
        self.internal_values
            .read()
            .get_f32(UnitFields::UnitFieldCombatReach.into())
    }

    pub fn is_attack_ready(&self, weap_type: WeaponAttackType) -> bool {
        let now = Instant::now();

        match weap_type {
            WeaponAttackType::MainHand => self.next_attack_times[weap_type as usize] <= now,
            WeaponAttackType::OffHand => {
                self.next_attack_times[weap_type as usize] <= now && self.has_off_hand
            }
            WeaponAttackType::Ranged => {
                error!("is_weapon_ready for Ranged NIY");
                false
            }
        }
    }

    pub fn reset_attack_type(&mut self, weap_type: WeaponAttackType) {
        self.next_attack_times[weap_type as usize] =
            Instant::now() + self.attack_intervals[weap_type as usize];
    }

    pub fn ensure_attack_time(&mut self, weap_type: WeaponAttackType, min_delay: Duration) {
        let current = &mut self.next_attack_times[weap_type as usize];
        *current = *current.max(&mut (Instant::now() + min_delay));
    }

    pub fn can_reach_target_in_melee(
        &self,
        my_position: &WorldPosition,
        target_position: &WorldPosition,
        target_melee_reach: f32,
    ) -> bool {
        let total_reach = self.melee_reach() + target_melee_reach + BASE_MELEE_RANGE_OFFSET;
        let distance = my_position.distance_to(target_position, true);

        distance <= total_reach
    }

    pub fn set_error(&mut self, error: MeleeAttackError, session: Option<Arc<WorldSession>>) {
        if error != self.last_error {
            self.last_error = error;

            match error {
                MeleeAttackError::NotInRange if session.is_some() => {
                    let packet = ServerMessage::new(SmsgAttackSwingNotInRange {});
                    session.unwrap().send(&packet).unwrap();
                }
                MeleeAttackError::NotFacingTarget => todo!(),
                _ => (),
            }
        }
    }

    pub fn set_sheath_state(&mut self, sheath_state: u32) {
        if let Some(sheath_state_enum) = SheathState::n(sheath_state) {
            self.internal_values.write().set_u8(
                UnitFields::UnitFieldBytes2.into(),
                0,
                sheath_state as u8,
            );

            self.sheath_state = sheath_state_enum;
        } else {
            warn!(
                "attempted to set an invalid sheath state ({}) on player",
                sheath_state
            );
        }
    }

    pub fn execute_attack(
        attacker_id: EntityId,
        map: Arc<Map>,
        data_store: Arc<DataStore>,
        v_guid: &View<Guid>,
        v_wpos: &View<WorldPosition>,
        v_spell: &View<SpellCast>,
        v_creature: &View<Creature>,
        v_unit: &mut ViewMut<Unit>,
        vm_powers: &mut ViewMut<Powers>,
        vm_melee: &mut ViewMut<Melee>,
        vm_threat_list: &mut ViewMut<ThreatList>,
        player_attacker: Option<&mut Player>,
    ) -> Result<(), ()> {
        let target_id = v_unit[attacker_id].target();

        if let Some(target_id) = target_id {
            if let Ok(target_guid) = v_guid.get(target_id).map(|g| g.0) {
                let guid = v_guid[attacker_id].0;
                let my_position = v_wpos[attacker_id];

                let mut target_powers = vm_powers
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

                let melee = &mut vm_melee.get(attacker_id).unwrap();
                let my_spell_cast = v_spell.get(attacker_id);

                if !melee.is_attacking
                    || my_spell_cast.is_ok_and(|sp| sp.current_ranged().is_some())
                {
                    return Err(());
                }

                if !target_powers.is_alive() {
                    let packet = {
                        ServerMessage::new(SmsgAttackStop {
                            attacker_guid: guid.as_packed(),
                            enemy_guid: target_guid.as_packed(),
                            unk: 0,
                        })
                    };

                    map.broadcast_packet(&guid, &packet, None, true);

                    melee.is_attacking = false;

                    return Err(());
                }

                if !melee.can_reach_target_in_melee(
                    &my_position,
                    target_position,
                    target_melee_reach,
                ) {
                    let my_session = map.get_session(&guid);
                    melee.set_error(MeleeAttackError::NotInRange, my_session);

                    melee
                        .ensure_attack_time(WeaponAttackType::MainHand, Duration::from_millis(100));
                    melee.ensure_attack_time(WeaponAttackType::OffHand, Duration::from_millis(100));
                    return Err(());
                }

                if melee.is_attack_ready(WeaponAttackType::MainHand) {
                    let damage = melee.calc_damage();
                    target_powers.apply_damage(damage as u32);

                    let packet = ServerMessage::new(SmsgAttackerStateUpdate {
                        hit_info: 2, // TODO enum HitInfo
                        attacker_guid: guid.as_packed(),
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

                    map.broadcast_packet(&guid, &packet, None, true);

                    melee.reset_attack_type(WeaponAttackType::MainHand);
                    melee.ensure_attack_time(WeaponAttackType::OffHand, ATTACK_DISPLAY_DELAY);
                    melee.set_error(MeleeAttackError::None, None);

                    if target_powers.is_alive() {
                        if let Ok(mut tl) = vm_threat_list.get(target_id) {
                            tl.modify_threat(attacker_id, damage);
                        }
                    } else if let Some(player) = player_attacker {
                        let mut has_loot = false; // TODO: Handle player case (Insignia looting in PvP)
                        if let Ok(creature) = v_creature.get(target_id) {
                            let xp_gain = Experience::xp_gain_against(
                                player,
                                creature,
                                map.id(),
                                data_store.clone(),
                            );
                            player.give_experience(xp_gain, Some(target_guid));
                            player.notify_killed_creature(creature.guid(), creature.template.entry);

                            has_loot = creature.generate_loot();
                        }

                        if let Ok(target_unit) = v_unit.get(target_id) {
                            if has_loot {
                                target_unit.set_dynamic_flag(UnitDynamicFlag::Lootable);
                            }
                        }

                        player.unset_in_combat_with(target_guid);
                    } else if let Ok(mut threat_list) = vm_threat_list.get(attacker_id) {
                        threat_list.remove(&target_id);
                    }

                    return Ok(());
                } else if melee.is_attack_ready(WeaponAttackType::OffHand) {
                    todo!();
                }
            }
        }

        Err(())
    }
}
