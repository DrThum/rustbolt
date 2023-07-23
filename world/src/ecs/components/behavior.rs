use std::time::Duration;

use shipyard::Component;

use crate::entities::behaviors::{BehaviorNode, BehaviorTree};

#[derive(Component)]
pub struct Behavior {
    bt: BehaviorTree<Action>,
}

impl Behavior {
    pub fn new_aggressive_monster() -> Self {
        let bt = BehaviorTree::new(BehaviorNode::new_cooldown_range_skippable(
            Box::new(BehaviorNode::Action(Action::WanderRandomly)),
            Duration::from_secs(3),
            Duration::from_secs(10),
            0.3,
        ));

        Self { bt }
    }

    pub fn tree(&mut self) -> &mut BehaviorTree<Action> {
        &mut self.bt
    }
}

pub enum Action {
    WanderRandomly,
}
