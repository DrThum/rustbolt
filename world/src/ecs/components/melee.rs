use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use log::error;
use shipyard::Component;

use crate::{
    entities::position::WorldPosition,
    protocol::{packets::SmsgAttackSwingNotInRange, server::ServerMessage},
    session::world_session::WorldSession,
    shared::constants::{
        MeleeAttackError, WeaponAttackType, BASE_MELEE_RANGE_OFFSET, NUMBER_WEAPON_ATTACK_TYPES,
    },
};

#[derive(Component)]
pub struct Melee {
    damage: u32,
    pub is_attacking: bool,
    next_attack_times: [Instant; NUMBER_WEAPON_ATTACK_TYPES], // MainHand, OffHand, Ranged
    attack_intervals: [Duration; NUMBER_WEAPON_ATTACK_TYPES],
    has_off_hand: bool,
    pub melee_reach: f32, // How far the unit can reach with its melee weapons
    last_error: MeleeAttackError,
}

impl Melee {
    pub fn new(damage: u32, melee_reach: f32) -> Self {
        let now = Instant::now();

        Self {
            damage,
            is_attacking: false,
            next_attack_times: [now, now, now],
            attack_intervals: [Duration::from_millis(800), Duration::MAX, Duration::MAX],
            has_off_hand: false,
            melee_reach,
            last_error: MeleeAttackError::None,
        }
    }

    pub fn damage(&self) -> u32 {
        self.damage
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
        let total_reach = self.melee_reach + target_melee_reach + BASE_MELEE_RANGE_OFFSET;
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
}
