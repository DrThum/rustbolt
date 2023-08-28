use std::time::Duration;

use shipyard::Component;

use crate::entities::behaviors::{BehaviorNode, BehaviorTree};

use super::movement::MovementKind;

#[derive(Component)]
pub struct Behavior {
    bt: BehaviorTree<Action>,
}

impl Behavior {
    pub fn new_wild_monster() -> Self {
        // Aggro an enemy entity passing by
        let aggro = BehaviorNode::new_cooldown(
            Box::new(BehaviorNode::Condition(
                Box::new(BehaviorNode::Action(Action::Aggro)),
                |ctx| {
                    // Check if we have a nearby player
                    // TODO: this should actually be "any enemy entity" not just "a player"
                    let curr_move_kind = **&ctx.vm_movement[ctx.entity_id].current_movement_kind();

                    curr_move_kind == MovementKind::Idle
                        || curr_move_kind.is_random()
                        || curr_move_kind == MovementKind::Path
                },
            )),
            Duration::from_millis(200),
        );

        let attack_melee = BehaviorNode::Action(Action::AttackInMelee);
        // let attack_spell = todo!();
        let chase_target = BehaviorNode::Action(Action::ChaseTarget);
        let attack =
            BehaviorNode::Selector(vec![attack_melee /*, attack_spell */, chase_target]);

        let bt = BehaviorTree::new(BehaviorNode::Selector(vec![attack, aggro]));

        Self { bt }
    }

    pub fn tree(&mut self) -> &mut BehaviorTree<Action> {
        &mut self.bt
    }
}

pub enum Action {
    Aggro,
    AttackInMelee,
    // AttackWithSpell,
    ChaseTarget,
}
