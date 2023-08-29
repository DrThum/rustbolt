use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use log::{error, warn};
use parking_lot::RwLock;
use shipyard::Component;

use crate::{
    entities::{
        internal_values::InternalValues, position::WorldPosition, update_fields::UnitFields,
    },
    protocol::{packets::SmsgAttackSwingNotInRange, server::ServerMessage},
    session::world_session::WorldSession,
    shared::constants::{
        MeleeAttackError, SheathState, WeaponAttackType, BASE_MELEE_RANGE_OFFSET,
        NUMBER_WEAPON_ATTACK_TYPES,
    },
};

#[derive(Component)]
pub struct Melee {
    internal_values: Arc<RwLock<InternalValues>>,
    damage: f32,
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
        damage: f32,
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
            guard.set_f32(UnitFields::UnitFieldMinDamage.into(), damage);
            guard.set_f32(UnitFields::UnitFieldMaxDamage.into(), damage);
        }

        Self {
            internal_values,
            damage,
            is_attacking: is_default_attacking,
            next_attack_times: [now, now, now],
            attack_intervals,
            has_off_hand: false,
            last_error: MeleeAttackError::None,
            sheath_state: SheathState::Unarmed,
        }
    }

    pub fn damage(&self) -> f32 {
        self.damage
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
}
