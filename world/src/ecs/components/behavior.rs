use std::time::Duration;

use shipyard::{Component, EntityId, View};

use crate::{
    datastore::data_types::FactionTemplateRecord,
    entities::behaviors::{BehaviorNode, BehaviorTree},
    shared::constants::FRIENDLY_FACTION_TEMPLATE_ID,
};

use super::{
    movement::{Movement, MovementKind},
    powers::Powers,
};

#[derive(Component)]
pub struct Behavior {
    bt: BehaviorTree<Action>,
    // Entities that moved around us within visibility distance during the last server tick
    moving_neighbors: Vec<EntityId>,
}

impl Behavior {
    pub fn new_wild_monster(faction_template: Option<&FactionTemplateRecord>) -> Self {
        // Aggro an enemy entity passing by
        let aggro = BehaviorNode::new_deadline(
            BehaviorNode::Condition(Box::new(BehaviorNode::Action(Action::Aggro)), |ctx| {
                ctx.all_storages.run(|v_movement: View<Movement>| {
                    let curr_move_kind = v_movement[ctx.entity_id].current_movement_kind();

                    *curr_move_kind == MovementKind::Idle
                        || curr_move_kind.is_random()
                        || *curr_move_kind == MovementKind::Path
                })
            }),
            Duration::from_millis(400),
            true,
        );

        let attack_melee = BehaviorNode::Action(Action::AttackInMelee);
        let chase_target = BehaviorNode::Action(Action::ChaseTarget);
        let attack = BehaviorNode::Selector(vec![attack_melee, chase_target]);

        let is_neutral_to_all = match faction_template {
            Some(ft) => ft.is_neutral_to_all() || ft.id == FRIENDLY_FACTION_TEMPLATE_ID,
            None => true,
        };
        // TODO: also exclude civilians and triggers

        let mut alive_actions = vec![attack];
        if !is_neutral_to_all {
            alive_actions.push(aggro);
        }

        let alive_behavior =
            BehaviorNode::Condition(Box::new(BehaviorNode::Selector(alive_actions)), |ctx| {
                ctx.all_storages
                    .run(|v_powers: View<Powers>| v_powers[ctx.entity_id].is_alive())
            });

        let respawn = BehaviorNode::Condition(
            Box::new(BehaviorNode::new_cooldown(
                BehaviorNode::Action(Action::Respawn),
                Duration::from_secs(5), // FIXME
                true,
            )),
            |ctx| {
                let is_alive = ctx
                    .all_storages
                    .run(|v_powers: View<Powers>| v_powers[ctx.entity_id].is_alive());
                !is_alive
            },
        );

        let bt = BehaviorTree::new(BehaviorNode::Selector(vec![alive_behavior, respawn]));

        Self {
            bt,
            moving_neighbors: Vec::new(),
        }
    }

    pub fn tree(&mut self) -> &mut BehaviorTree<Action> {
        &mut self.bt
    }

    pub fn neighbor_moved(&mut self, neighbor_entity_id: EntityId) {
        self.moving_neighbors.push(neighbor_entity_id);
    }

    pub fn neighbors(&self) -> Vec<EntityId> {
        self.moving_neighbors.to_vec()
    }

    pub fn reset_neighbors(&mut self) {
        self.moving_neighbors.clear();
    }
}

pub enum Action {
    Aggro,
    AttackInMelee,
    // AttackWithSpell,
    ChaseTarget,
    Respawn,
}
