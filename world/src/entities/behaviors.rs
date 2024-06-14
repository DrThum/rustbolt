use std::time::{Duration, Instant};

use rand::Rng;
use shipyard::{AllStoragesViewMut, EntityId};

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
            BehaviorNodeState::Deadline {
                ref mut child,
                delay_min,
                delay_max,
                skip_chance,
                ignore_status_for_reset,
                ref mut expire_time,
            } => {
                let now = Instant::now();
                if *expire_time <= now {
                    let res = Self::evaluate_tree(child, dt, ctx, tick_fn);

                    if *ignore_status_for_reset || res == NodeStatus::Success {
                        let mut rng = rand::thread_rng();

                        // Reset the deadline according to the skip chance
                        if *skip_chance == 0. || rng.gen_range(0.0..1.0) > *skip_chance {
                            let next_deadline = if *delay_min == *delay_max {
                                *delay_min
                            } else {
                                rng.gen_range(*delay_min..*delay_max)
                            };
                            *expire_time = now + next_deadline;
                        }
                    }

                    res
                } else {
                    NodeStatus::Failure
                }
            }
            BehaviorNodeState::Cooldown {
                ref mut child,
                cooldown_min,
                cooldown_max,
                skip_chance,
                ignore_status_for_reset,
                ref mut cooldown_left,
            } => {
                if *cooldown_left <= dt {
                    let res = Self::evaluate_tree(child, dt, ctx, tick_fn);

                    if *ignore_status_for_reset || res == NodeStatus::Success {
                        let mut rng = rand::thread_rng();

                        // Reset the deadline according to the skip chance
                        if *skip_chance == 0. || rng.gen_range(0.0..1.0) > *skip_chance {
                            let next_cooldown = if *cooldown_min == *cooldown_max {
                                *cooldown_min
                            } else {
                                rng.gen_range(*cooldown_min..*cooldown_max)
                            };
                            *cooldown_left = next_cooldown;
                        }
                    }

                    res
                } else {
                    *cooldown_left -= dt;
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
    // The difference with Cooldown is that Deadline sets a fixed moment in the future when the
    // subtree will be unlocked, regardless of any other element (a Condition node wrapping this
    // one for example)
    Deadline(
        Box<BehaviorNode<A>>, // Child
        Duration,             // Minimum delay (if range)
        Duration,             // Maximum delay (if range)
        bool,                 // True if the cooldown should reset no matter the child status,
        // false if it should reset only when child status is Success
        f32, // Chance to skip and unlock immediately [0..1[
    ),
    // Decorator that locks the child tree for the configured time after it has returned Success
    // The difference with Deadline is that Cooldown sets an amount of time that is decremented
    // every time the node is evaluated. This means that if the Cooldown node is wrapped in a
    // Condition node for example, the cooldown is only reduced when the Condition is true
    Cooldown(
        Box<BehaviorNode<A>>, // Child
        Duration,             // Minimum cooldown (if range)
        Duration,             // Maximum cooldown (if range)
        bool,                 // True if the cooldown should reset no matter the child status,
        // false if it should reset only when child status is Success
        f32, // Chance to skip and unlock immediately [0..1[
    ),
    // Decorator that executes the child tree if the predicate is true
    Condition(Box<BehaviorNode<A>>, fn(&BTContext) -> bool),
    // Actual action node, leaf on the tree (implemented by the user in `tick_fn`)
    Action(A),
}

#[allow(dead_code)]
impl<A> BehaviorNode<A> {
    pub fn new_deadline(
        child: Box<BehaviorNode<A>>,
        delay: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Deadline(child, delay, delay, ignore_status_for_reset, 0.)
    }

    pub fn new_deadline_skippable(
        child: Box<BehaviorNode<A>>,
        delay: Duration,
        skip_chance: f32,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Deadline(child, delay, delay, ignore_status_for_reset, skip_chance)
    }

    pub fn new_deadline_range(
        child: Box<BehaviorNode<A>>,
        min: Duration,
        max: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Deadline(child, min, max, ignore_status_for_reset, 0.)
    }

    pub fn new_deadline_range_skippable(
        child: Box<BehaviorNode<A>>,
        min: Duration,
        max: Duration,
        ignore_status_for_reset: bool,
        skip_chance: f32,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Deadline(child, min, max, ignore_status_for_reset, skip_chance)
    }

    pub fn new_cooldown(
        child: Box<BehaviorNode<A>>,
        cooldown: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Cooldown(child, cooldown, cooldown, ignore_status_for_reset, 0.)
    }

    pub fn new_cooldown_skippable(
        child: Box<BehaviorNode<A>>,
        cooldown: Duration,
        skip_chance: f32,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Cooldown(
            child,
            cooldown,
            cooldown,
            ignore_status_for_reset,
            skip_chance,
        )
    }

    pub fn new_cooldown_range(
        child: Box<BehaviorNode<A>>,
        min: Duration,
        max: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Cooldown(child, min, max, ignore_status_for_reset, 0.)
    }

    pub fn new_cooldown_range_skippable(
        child: Box<BehaviorNode<A>>,
        min: Duration,
        max: Duration,
        skip_chance: f32,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Cooldown(child, min, max, ignore_status_for_reset, skip_chance)
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
            BehaviorNode::Deadline(
                child,
                delay_min,
                delay_max,
                ignore_status_for_reset,
                skip_chance,
            ) => BehaviorNodeState::Deadline {
                child: Box::new(child.build_state()),
                delay_min,
                delay_max,
                skip_chance,
                ignore_status_for_reset,
                expire_time: Instant::now(),
            },
            BehaviorNode::Cooldown(
                child,
                cooldown_min,
                cooldown_max,
                ignore_status_for_reset,
                skip_chance,
            ) => {
                BehaviorNodeState::Cooldown {
                    child: Box::new(child.build_state()),
                    cooldown_min,
                    cooldown_max,
                    skip_chance,
                    ignore_status_for_reset,
                    cooldown_left: cooldown_min, // FIXME: we might want to be random here
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

pub struct BTContext<'a> {
    pub entity_id: EntityId,
    pub neighbors: Vec<EntityId>,
    pub all_storages: &'a AllStoragesViewMut<'a>,
}

// Internals
enum BehaviorNodeState<A> {
    Sequence(Vec<BehaviorNodeState<A>>),
    Selector(Vec<BehaviorNodeState<A>>),
    Deadline {
        child: Box<BehaviorNodeState<A>>,
        delay_min: Duration,
        delay_max: Duration,
        skip_chance: f32,
        // If true, the delay is reset no matter the NodeStatus of the child node
        // If false, the delay is reset only if the child returns NodeStatus::Success
        ignore_status_for_reset: bool,
        expire_time: Instant, // When will the subtree be unlocked?
    },
    Cooldown {
        child: Box<BehaviorNodeState<A>>,
        cooldown_min: Duration,
        cooldown_max: Duration,
        skip_chance: f32,
        // If true, the delay is reset no matter the NodeStatus of the child node
        // If false, the delay is reset only if the child returns NodeStatus::Success
        ignore_status_for_reset: bool,
        cooldown_left: Duration, // Time left on the cooldown
    },
    Condition(Box<BehaviorNodeState<A>>, fn(&BTContext) -> bool),
    Action(A),
}
