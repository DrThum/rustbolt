use std::time::{Duration, Instant};

use parking_lot::{RwLock, RwLockWriteGuard};
use rand::Rng;
use shipyard::{AllStoragesViewMut, EntityId};

#[allow(dead_code)]
pub struct BehaviorTree<A> {
    nodes: Vec<RwLock<BehaviorNodeState<A>>>,
    running_node_index: Option<usize>,
    root_index: usize,
}

impl<A> BehaviorTree<A> {
    pub fn new(root: BehaviorNode<A>) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            running_node_index: None,
            root_index: 0,
        };
        let root_index = root.build_state(&mut tree);
        tree.root_index = root_index;
        tree
    }

    fn node_by_index_mut(&self, index: usize) -> RwLockWriteGuard<BehaviorNodeState<A>> {
        self.nodes[index].write()
    }

    fn register_node(&mut self, node: BehaviorNodeState<A>) -> usize {
        self.nodes.push(RwLock::new(node));
        self.nodes.len() - 1
    }

    pub fn tick(
        &mut self,
        dt: Duration,
        ctx: &mut BTContext,
        tick_fn: fn(&A, &mut BTContext) -> NodeStatus,
    ) {
        self.evaluate_tree(self.root_index, dt, ctx, tick_fn);
    }

    fn evaluate_tree(
        &self,
        node_index: usize,
        dt: Duration,
        ctx: &mut BTContext,
        tick_fn: fn(&A, &mut BTContext) -> NodeStatus,
    ) -> NodeStatus {
        match &mut *self.node_by_index_mut(node_index) {
            BehaviorNodeState::Sequence(nodes) => {
                for child_node_index in nodes.clone() {
                    let status = self.evaluate_tree(child_node_index, dt, ctx, tick_fn);
                    if status != NodeStatus::Success {
                        return status;
                    }
                }
                NodeStatus::Success
            }
            BehaviorNodeState::Selector(nodes) => {
                for child_node_index in nodes.clone() {
                    let status = self.evaluate_tree(child_node_index, dt, ctx, tick_fn);
                    if status != NodeStatus::Failure {
                        return status;
                    }
                }
                NodeStatus::Failure
            }
            BehaviorNodeState::Deadline {
                child,
                delay_min,
                delay_max,
                skip_chance,
                ignore_status_for_reset,
                ref mut expire_time,
            } => {
                let now = Instant::now();
                if *expire_time <= now {
                    let res = self.evaluate_tree(*child, dt, ctx, tick_fn);

                    if *ignore_status_for_reset || res == NodeStatus::Success {
                        let mut rng = rand::thread_rng();

                        // Reset the deadline according to the skip chance
                        if *skip_chance == 0. || rng.gen_range(0.0..1.0) > *skip_chance {
                            let next_deadline = if delay_min == delay_max {
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
                child,
                cooldown_min,
                cooldown_max,
                skip_chance,
                ignore_status_for_reset,
                ref mut cooldown_left,
            } => {
                if *cooldown_left <= dt {
                    let res = self.evaluate_tree(*child, dt, ctx, tick_fn);

                    if *ignore_status_for_reset || res == NodeStatus::Success {
                        let mut rng = rand::thread_rng();

                        // Reset the deadline according to the skip chance
                        if *skip_chance == 0. || rng.gen_range(0.0..1.0) > *skip_chance {
                            let next_cooldown = if cooldown_min == cooldown_max {
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
                    self.evaluate_tree(*child, dt, ctx, tick_fn)
                } else {
                    NodeStatus::Failure
                }
            }
            BehaviorNodeState::Action(action) => tick_fn(&action, ctx),
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
        child: BehaviorNode<A>,
        delay: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Deadline(Box::new(child), delay, delay, ignore_status_for_reset, 0.)
    }

    pub fn new_deadline_skippable(
        child: BehaviorNode<A>,
        delay: Duration,
        skip_chance: f32,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Deadline(
            Box::new(child),
            delay,
            delay,
            ignore_status_for_reset,
            skip_chance,
        )
    }

    pub fn new_deadline_range(
        child: BehaviorNode<A>,
        min: Duration,
        max: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Deadline(Box::new(child), min, max, ignore_status_for_reset, 0.)
    }

    pub fn new_deadline_range_skippable(
        child: BehaviorNode<A>,
        min: Duration,
        max: Duration,
        ignore_status_for_reset: bool,
        skip_chance: f32,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Deadline(
            Box::new(child),
            min,
            max,
            ignore_status_for_reset,
            skip_chance,
        )
    }

    pub fn new_cooldown(
        child: BehaviorNode<A>,
        cooldown: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Cooldown(
            Box::new(child),
            cooldown,
            cooldown,
            ignore_status_for_reset,
            0.,
        )
    }

    pub fn new_cooldown_skippable(
        child: BehaviorNode<A>,
        cooldown: Duration,
        skip_chance: f32,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Cooldown(
            Box::new(child),
            cooldown,
            cooldown,
            ignore_status_for_reset,
            skip_chance,
        )
    }

    pub fn new_cooldown_range(
        child: BehaviorNode<A>,
        min: Duration,
        max: Duration,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        Self::Cooldown(Box::new(child), min, max, ignore_status_for_reset, 0.)
    }

    pub fn new_cooldown_range_skippable(
        child: BehaviorNode<A>,
        min: Duration,
        max: Duration,
        skip_chance: f32,
        ignore_status_for_reset: bool,
    ) -> BehaviorNode<A> {
        assert!(
            skip_chance >= 0. && skip_chance < 1.,
            "skip_chance must be [0..1["
        );
        Self::Cooldown(
            Box::new(child),
            min,
            max,
            ignore_status_for_reset,
            skip_chance,
        )
    }
}

impl<A> BehaviorNode<A> {
    fn build_state(self, tree: &mut BehaviorTree<A>) -> usize {
        match self {
            BehaviorNode::Sequence(children) => {
                let node = BehaviorNodeState::Sequence(
                    children.into_iter().map(|c| c.build_state(tree)).collect(),
                );
                tree.register_node(node)
            }
            BehaviorNode::Selector(children) => {
                let node = BehaviorNodeState::Selector(
                    children.into_iter().map(|c| c.build_state(tree)).collect(),
                );
                tree.register_node(node)
            }
            BehaviorNode::Deadline(
                child,
                delay_min,
                delay_max,
                ignore_status_for_reset,
                skip_chance,
            ) => {
                let node = BehaviorNodeState::Deadline {
                    child: child.build_state(tree),
                    delay_min,
                    delay_max,
                    skip_chance,
                    ignore_status_for_reset,
                    expire_time: Instant::now(),
                };
                tree.register_node(node)
            }
            BehaviorNode::Cooldown(
                child,
                cooldown_min,
                cooldown_max,
                ignore_status_for_reset,
                skip_chance,
            ) => {
                let node = BehaviorNodeState::Cooldown {
                    child: child.build_state(tree),
                    cooldown_min,
                    cooldown_max,
                    skip_chance,
                    ignore_status_for_reset,
                    cooldown_left: cooldown_min, // FIXME: we might want to be random here
                };
                tree.register_node(node)
            }
            BehaviorNode::Condition(child, pred) => {
                let node = BehaviorNodeState::Condition(child.build_state(tree), pred);
                tree.register_node(node)
            }
            BehaviorNode::Action(action) => {
                let node = BehaviorNodeState::Action(action);
                tree.register_node(node)
            }
        }
    }
}

#[derive(PartialEq, Debug)]
#[allow(dead_code)]
pub enum NodeStatus {
    Success,
    Failure,
    Running(usize),
}

pub struct BTContext<'a> {
    pub entity_id: EntityId,
    pub neighbors: Vec<EntityId>,
    pub all_storages: &'a AllStoragesViewMut<'a>,
}

// Internals
enum BehaviorNodeState<A> {
    Sequence(Vec<usize>),
    Selector(Vec<usize>),
    Deadline {
        child: usize,
        delay_min: Duration,
        delay_max: Duration,
        skip_chance: f32,
        // If true, the delay is reset no matter the NodeStatus of the child node
        // If false, the delay is reset only if the child returns NodeStatus::Success
        ignore_status_for_reset: bool,
        expire_time: Instant, // When will the subtree be unlocked?
    },
    Cooldown {
        child: usize,
        cooldown_min: Duration,
        cooldown_max: Duration,
        skip_chance: f32,
        // If true, the delay is reset no matter the NodeStatus of the child node
        // If false, the delay is reset only if the child returns NodeStatus::Success
        ignore_status_for_reset: bool,
        cooldown_left: Duration, // Time left on the cooldown
    },
    Condition(usize, fn(&BTContext) -> bool),
    Action(A),
}
