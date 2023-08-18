use std::time::{Duration, Instant};

use rand::Rng;
use shipyard::{EntityId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{guid::Guid, movement::Movement, unit::Unit},
        resources::DeltaTime,
    },
    game::map::WrappedMap,
};

use super::{creature::Creature, player::Player, position::WorldPosition};

#[allow(dead_code)]
pub struct BehaviorTree<A> {
    root: BehaviorNodeState<A>,
}

impl<A> BehaviorTree<A> {
    pub fn new(root: BehaviorNode<A>) -> Self {
        Self {
            root: root.build_state(),
        }
    }

    pub fn tick(
        &mut self,
        dt: Duration,
        ctx: &mut BTContext,
        tick_fn: fn(&A, &mut BTContext) -> NodeStatus,
    ) {
        Self::evaluate_tree(&mut self.root, dt, ctx, tick_fn);
    }

    fn evaluate_tree(
        node: &mut BehaviorNodeState<A>,
        dt: Duration,
        ctx: &mut BTContext,
        tick_fn: fn(&A, &mut BTContext) -> NodeStatus,
    ) -> NodeStatus {
        match node {
            BehaviorNodeState::Sequence(nodes) => {
                for child_node in nodes {
                    let status = Self::evaluate_tree(child_node, dt, ctx, tick_fn);
                    if status != NodeStatus::Success {
                        return status;
                    }
                }
                NodeStatus::Success
            }
            BehaviorNodeState::Selector(nodes) => {
                for child_node in nodes {
                    let status = Self::evaluate_tree(child_node, dt, ctx, tick_fn);
                    if status == NodeStatus::Success {
                        return status;
                    }
                }
                NodeStatus::Failure
            }
            BehaviorNodeState::Cooldown {
                ref mut child,
                ref_cooldown_min,
                ref_cooldown_max,
                skip_chance,
                ref mut expire_time,
            } => {
                let now = Instant::now();
                if *expire_time <= now {
                    let res = Self::evaluate_tree(child, dt, ctx, tick_fn);

                    if res == NodeStatus::Success {
                        let mut rng = rand::thread_rng();

                        // Reset the cooldown according to the skip chance
                        if rng.gen_range(0.0..1.0) > *skip_chance {
                            let next_cooldown = rng.gen_range(*ref_cooldown_min..*ref_cooldown_max);
                            *expire_time = now + next_cooldown;
                        }
                    }

                    res
                } else {
                    NodeStatus::Failure
                }
            }
            BehaviorNodeState::Condition(child, pred) => {
                if pred(ctx) {
                    Self::evaluate_tree(child, dt, ctx, tick_fn)
                } else {
                    NodeStatus::Failure
                }
            }
            BehaviorNodeState::Action(action) => tick_fn(action, ctx),
        }
    }
}

#[allow(dead_code)]
pub enum BehaviorNode<A> {
    // Node that returns Success if all its children return Success, Failure otherwise
    Sequence(Vec<BehaviorNode<A>>),
    // Node that returns Success after the first child returns Success. If all children return
    // Failure, then returns Failure too.
    Selector(Vec<BehaviorNode<A>>),
    // Decorator that locks the child tree for the configured time after it has returned Success
    Cooldown(
        Box<BehaviorNode<A>>, // Child
        Duration,             // Minimum duration (if range)
        Duration,             // Maximum duration (if range)
        f32,                  // Chance to skip cooldown [0..1[
    ),
    // Decorator that executes the child tree if the predicate is true
    Condition(Box<BehaviorNode<A>>, fn(&BTContext) -> bool),
    // Actual action node, leaf on the tree (implemented by the user in `tick_fn`)
    Action(A),
}

#[allow(dead_code)]
impl<A> BehaviorNode<A> {
    pub fn new_cooldown(child: Box<BehaviorNode<A>>, cooldown: Duration) -> BehaviorNode<A> {
        Self::Cooldown(child, cooldown, cooldown, 0.)
    }

    pub fn new_cooldown_skippable(
        child: Box<BehaviorNode<A>>,
        cooldown: Duration,
        skip_chance: f32,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Cooldown(child, cooldown, cooldown, skip_chance)
    }

    pub fn new_cooldown_range(
        child: Box<BehaviorNode<A>>,
        min: Duration,
        max: Duration,
    ) -> BehaviorNode<A> {
        Self::Cooldown(child, min, max, 0.)
    }

    pub fn new_cooldown_range_skippable(
        child: Box<BehaviorNode<A>>,
        min: Duration,
        max: Duration,
        skip_chance: f32,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Cooldown(child, min, max, skip_chance)
    }
}

impl<A> BehaviorNode<A> {
    fn build_state(self) -> BehaviorNodeState<A> {
        match self {
            BehaviorNode::Sequence(children) => {
                BehaviorNodeState::Sequence(children.into_iter().map(|c| c.build_state()).collect())
            }
            BehaviorNode::Selector(children) => {
                BehaviorNodeState::Selector(children.into_iter().map(|c| c.build_state()).collect())
            }
            BehaviorNode::Cooldown(child, ref_cooldown_min, ref_cooldown_max, skip_chance) => {
                BehaviorNodeState::Cooldown {
                    child: Box::new(child.build_state()),
                    ref_cooldown_min,
                    ref_cooldown_max,
                    skip_chance,
                    expire_time: Instant::now(),
                }
            }
            BehaviorNode::Condition(child, pred) => {
                BehaviorNodeState::Condition(Box::new(child.build_state()), pred)
            }
            BehaviorNode::Action(action) => BehaviorNodeState::Action(action),
        }
    }
}

#[derive(PartialEq, Debug)]
#[allow(dead_code)]
pub enum NodeStatus {
    Success,
    Failure,
    Running,
}

pub struct BTContext<'a, 'b, 'c> {
    pub entity_id: EntityId,
    pub dt: &'a UniqueView<'a, DeltaTime>,
    pub map: &'a UniqueView<'a, WrappedMap>,
    pub vm_movement: &'a mut ViewMut<'b, Movement>,
    pub v_guid: &'a View<'a, Guid>,
    pub v_wpos: &'a View<'a, WorldPosition>,
    pub v_creature: &'a View<'a, Creature>,
    pub v_player: &'a View<'a, Player>,
    pub vm_unit: &'a mut ViewMut<'c, Unit>,
}

// Internals
enum BehaviorNodeState<A> {
    Sequence(Vec<BehaviorNodeState<A>>),
    Selector(Vec<BehaviorNodeState<A>>),
    Cooldown {
        child: Box<BehaviorNodeState<A>>,
        ref_cooldown_min: Duration,
        ref_cooldown_max: Duration,
        skip_chance: f32,
        expire_time: Instant, // When will the cooldown be up?
    },
    Condition(Box<BehaviorNodeState<A>>, fn(&BTContext) -> bool),
    Action(A),
}
