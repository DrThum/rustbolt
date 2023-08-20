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
        // TODO: Add a cooldown around this one? Like 200 ms?
        let aggro = BehaviorNode::Condition(Box::new(BehaviorNode::Action(Action::Aggro)), |ctx| {
            // Check if we have a nearby player
            // TODO: this should actually be "any enemy entity" not just "a player"
            // TODO: we should only react if we're hostile to the target
            let curr_move_kind = **&ctx.vm_movement[ctx.entity_id].current_movement_kind();

            curr_move_kind == MovementKind::Idle
                || curr_move_kind.is_random()
                || curr_move_kind == MovementKind::Path
        });

        let bt = BehaviorTree::new(BehaviorNode::Selector(vec![aggro]));

        Self { bt }
    }

    pub fn tree(&mut self) -> &mut BehaviorTree<Action> {
        &mut self.bt
    }
}

pub enum Action {
    Aggro,
}
