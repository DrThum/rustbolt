use std::collections::HashMap;

use log::warn;
use shared::models::terrain_info::MAP_MAX_COORD;

use crate::entities::{object_guid::ObjectGuid, position::Position};

// QuadTree to store entities position in the world
// #[derive(Debug, Clone)]
// struct GuidPosition {
//     // TODO: use refs?
//     pub guid: ObjectGuid,
//     pub pos: Position,
// }
//
// impl GuidPosition {
//     pub fn point(&self) -> Point {
//         Point {
//             x: self.pos.x,
//             y: self.pos.y,
//         }
//     }
// }

#[derive(Debug)]
enum NodeContent {
    Empty,
    Values(Vec<ObjectGuid>),
    Children {
        nw: Box<Node>,
        ne: Box<Node>,
        sw: Box<Node>,
        se: Box<Node>,
    },
}

#[derive(PartialEq, Debug)]
enum Quadrant {
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}

impl Quadrant {
    // Reminder: in WoW coords, X grows upward and y grows leftward
    fn select(coords: &Position, bounds: &Bounds) -> Self {
        let mid_x = (bounds.upper_left.x + bounds.lower_right.x) / 2.0;
        let mid_y = (bounds.upper_left.y + bounds.lower_right.y) / 2.0;

        match (coords.x <= mid_x, coords.y <= mid_y) {
            (true, true) => Quadrant::SouthEast,
            (true, false) => Quadrant::SouthWest,
            (false, true) => Quadrant::NorthEast,
            (false, false) => Quadrant::NorthWest,
        }
    }
}

struct Node {
    content: NodeContent,
    bounds: Bounds,
}

impl core::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.content {
            NodeContent::Empty => write!(f, "[_]"),
            NodeContent::Values(vs) => {
                write!(f, "{:?}", vs.iter().map(|v| v.raw()).collect::<Vec<u64>>())
            }
            NodeContent::Children { nw, ne, sw, se } => {
                write!(f, "[NW{:?},NE{:?},SW{:?},SE{:?}]", nw, ne, sw, se)
            }
        }
    }
}

impl Node {
    pub fn leaf(value: ObjectGuid, bounds: Bounds) -> Node {
        Node {
            content: NodeContent::Values(vec![value]),
            bounds,
        }
    }

    pub fn leaf_or_empty(values: Vec<ObjectGuid>, bounds: Bounds) -> Node {
        if values.len() > 0 {
            Node {
                content: NodeContent::Values(values),
                bounds,
            }
        } else {
            Node::empty(bounds)
        }
    }

    pub fn empty(bounds: Bounds) -> Node {
        Node {
            content: NodeContent::Empty,
            bounds,
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.content {
            NodeContent::Empty => true,
            NodeContent::Values(vs) => vs.is_empty(),
            NodeContent::Children { nw, ne, sw, se } => {
                nw.is_empty() && ne.is_empty() && sw.is_empty() && se.is_empty()
            }
        }
    }
}

pub const QUADTREE_DEFAULT_NODE_CAPACITY: usize = 100;

pub struct QuadTree {
    node_capacity: usize,
    root: Box<Node>,
    entities_positions: HashMap<ObjectGuid, Position>,
}

impl core::fmt::Debug for QuadTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QuadTree({})<{:?}>", self.node_capacity, self.root)
    }
}

impl QuadTree {
    pub fn new(node_capacity: usize) -> Self {
        let root_bounds = Bounds {
            upper_left: Point {
                x: MAP_MAX_COORD,
                y: MAP_MAX_COORD,
            },
            lower_right: Point {
                x: -MAP_MAX_COORD,
                y: -MAP_MAX_COORD,
            },
        };

        Self {
            node_capacity,
            root: Box::new(Node::empty(root_bounds)),
            entities_positions: HashMap::new(),
        }
    }

    pub fn insert(&mut self, pos: Position, guid: ObjectGuid) {
        fn insert_rec(
            entities_positions: &HashMap<ObjectGuid, Position>,
            node: &mut Box<Node>,
            new_value: ObjectGuid,
            new_value_position: Position,
            node_capacity: usize,
        ) {
            match &mut (*node).content {
                // Node is empty, transform it to a Leaf node with the new value
                NodeContent::Empty => **node = Node::leaf(new_value, node.bounds.clone()),
                // Node is a leaf but is not at capacity yet, add the new value
                NodeContent::Values(ref mut existing_values)
                    if existing_values.len() < node_capacity =>
                {
                    existing_values.push(new_value)
                }
                // Node is a full leaf, subdivide it in four then add the new value
                NodeContent::Values(ref mut existing_values) => {
                    // Subdivide...
                    let mut nw: Vec<ObjectGuid> = Vec::new();
                    let mut ne: Vec<ObjectGuid> = Vec::new();
                    let mut sw: Vec<ObjectGuid> = Vec::new();
                    let mut se: Vec<ObjectGuid> = Vec::new();

                    for value in existing_values {
                        let position = entities_positions
                            .get(&value)
                            .expect("entity exists in quadtree but is not in entities_positions");

                        match Quadrant::select(&position, &node.bounds) {
                            Quadrant::NorthWest => nw.push(value.clone()),
                            Quadrant::NorthEast => ne.push(value.clone()),
                            Quadrant::SouthWest => sw.push(value.clone()),
                            Quadrant::SouthEast => se.push(value.clone()),
                        }
                    }

                    // ...then insert in the relevant quadrant
                    match Quadrant::select(&new_value_position, &node.bounds) {
                        Quadrant::NorthWest => nw.push(new_value),
                        Quadrant::NorthEast => ne.push(new_value),
                        Quadrant::SouthWest => sw.push(new_value),
                        Quadrant::SouthEast => se.push(new_value),
                    };

                    let nw = Node::leaf_or_empty(
                        nw,
                        Bounds::for_quadrant(&node.bounds, Quadrant::NorthWest),
                    );
                    let ne = Node::leaf_or_empty(
                        ne,
                        Bounds::for_quadrant(&node.bounds, Quadrant::NorthEast),
                    );
                    let sw = Node::leaf_or_empty(
                        sw,
                        Bounds::for_quadrant(&node.bounds, Quadrant::SouthWest),
                    );
                    let se = Node::leaf_or_empty(
                        se,
                        Bounds::for_quadrant(&node.bounds, Quadrant::SouthEast),
                    );

                    **node = Node {
                        content: NodeContent::Children {
                            nw: Box::new(nw),
                            ne: Box::new(ne),
                            sw: Box::new(sw),
                            se: Box::new(se),
                        },
                        bounds: node.bounds.clone(),
                    };
                }
                // Node is an internal one, recursively insert into the relevant quadrant
                NodeContent::Children { nw, ne, sw, se } => {
                    let new_value_quadrant = Quadrant::select(&new_value_position, &node.bounds);

                    let target_quadrant = match new_value_quadrant {
                        Quadrant::NorthWest => nw,
                        Quadrant::NorthEast => ne,
                        Quadrant::SouthWest => sw,
                        Quadrant::SouthEast => se,
                    };

                    insert_rec(
                        entities_positions,
                        target_quadrant,
                        new_value,
                        new_value_position,
                        node_capacity,
                    );
                }
            }
        }

        insert_rec(
            &self.entities_positions,
            &mut self.root,
            guid,
            pos,
            self.node_capacity,
        );
        self.entities_positions.insert(guid, pos);
    }

    pub fn search_around_position(
        &self,
        position: &Position,
        radius: f32,
        search_in_3d: bool,
    ) -> Vec<ObjectGuid> {
        fn search_rec(
            entities_positions: &HashMap<ObjectGuid, Position>,
            node: &Box<Node>,
            center: &Position,
            radius: f32,
            search_in_3d: bool,
            acc: &mut Vec<ObjectGuid>,
        ) {
            match &node.content {
                NodeContent::Empty => (),
                NodeContent::Values(values) if node.bounds.intersects_circle(&center, radius) => {
                    let radius_square = radius * radius;
                    for value in values {
                        let position = entities_positions
                            .get(&value)
                            .expect("entity exists in quadtree but is not in entities_positions");
                        let dist_square = if search_in_3d {
                            position.square_distance_3d(&center)
                        } else {
                            position.square_distance_2d(&center)
                        };

                        if dist_square <= radius_square {
                            acc.push(value.clone());
                        }
                    }
                }
                NodeContent::Values(_) => (),
                NodeContent::Children { nw, ne, sw, se } => {
                    if nw.bounds.intersects_circle(&center, radius) {
                        search_rec(entities_positions, nw, &center, radius, search_in_3d, acc);
                    }
                    if ne.bounds.intersects_circle(&center, radius) {
                        search_rec(entities_positions, ne, &center, radius, search_in_3d, acc);
                    }
                    if sw.bounds.intersects_circle(&center, radius) {
                        search_rec(entities_positions, sw, &center, radius, search_in_3d, acc);
                    }
                    if se.bounds.intersects_circle(&center, radius) {
                        search_rec(entities_positions, se, &center, radius, search_in_3d, acc);
                    }
                }
            }
        }

        let mut guids: Vec<ObjectGuid> = Vec::new();
        search_rec(
            &self.entities_positions,
            &self.root,
            &position,
            radius,
            search_in_3d,
            &mut guids,
        );
        guids
    }

    // pub fn search_around_position(&self, position: &Position, radius: f32, search_in_3d: bool) -> Vec<ObjectGuid> {
    pub fn search_around_entity(
        &self,
        guid: &ObjectGuid,
        radius: f32,
        search_in_3d: bool,
    ) -> Vec<ObjectGuid> {
        if let Some(position) = self.entities_positions.get(&guid) {
            return self.search_around_position(position, radius, search_in_3d);
        }

        warn!("QuadTree::search_around_entity: searching for entity with guid {} that is not present in entities_position", guid.raw());
        Vec::new()
    }

    pub fn delete(&mut self, guid: &ObjectGuid) {
        fn delete_rec(node: &mut Box<Node>, position: &Position, value: &ObjectGuid) {
            match &mut (*node).content {
                NodeContent::Values(ref mut existing_values) => {
                    existing_values.retain(|v| v != value);

                    if existing_values.is_empty() {
                        **node = Node::empty(node.bounds.clone());
                    }
                }
                NodeContent::Children { nw, ne, sw, se } => {
                    let value_quadrant = Quadrant::select(position, &node.bounds);

                    match value_quadrant {
                        Quadrant::NorthWest => delete_rec(nw, position, value),
                        Quadrant::NorthEast => delete_rec(ne, position, value),
                        Quadrant::SouthWest => delete_rec(sw, position, value),
                        Quadrant::SouthEast => delete_rec(se, position, value),
                    };

                    if node.is_empty() {
                        **node = Node::empty(node.bounds.clone());
                    }
                }
                _ => (),
            }
        }

        let position = self
            .entities_positions
            .get(&guid)
            .expect("entity exists in quadtree but is not in entities_positions");
        delete_rec(&mut self.root, position, &guid);
        self.entities_positions.remove(&guid);
    }

    pub fn update(&mut self, new_position: &Position, guid: &ObjectGuid) {
        // Possible optimization: search for the value and update it in place if the new position
        // ends up in the same node as the old position
        // For now, simply delete then insert
        self.delete(guid);
        self.insert(new_position.clone(), guid.clone());
    }
}

impl Position {
    pub fn square_distance_2d(&self, other: &Position) -> f32 {
        let dist_x = self.x - other.x;
        let dist_y = self.y - other.y;

        (dist_x * dist_x) + (dist_y * dist_y)
    }

    pub fn square_distance_3d(&self, other: &Position) -> f32 {
        let dist_x = self.x - other.x;
        let dist_y = self.y - other.y;
        let dist_z = self.z - other.z;

        (dist_x * dist_x) + (dist_y * dist_y) + (dist_z * dist_z)
    }
}

#[derive(Clone, Debug)]
struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone)]
struct Bounds {
    upper_left: Point,
    lower_right: Point,
}

impl Bounds {
    fn for_quadrant(bounds: &Bounds, quadrant: Quadrant) -> Self {
        let mid_point = Point {
            x: (bounds.upper_left.x + bounds.lower_right.x) / 2.0,
            y: (bounds.upper_left.y + bounds.lower_right.y) / 2.0,
        };

        match quadrant {
            Quadrant::NorthWest => Bounds {
                upper_left: bounds.upper_left.clone(),
                lower_right: mid_point.clone(),
            },
            Quadrant::NorthEast => Bounds {
                upper_left: Point {
                    x: bounds.upper_left.x,
                    y: mid_point.y,
                },
                lower_right: Point {
                    x: mid_point.x,
                    y: bounds.lower_right.y,
                },
            },
            Quadrant::SouthWest => Bounds {
                upper_left: Point {
                    x: mid_point.x,
                    y: bounds.upper_left.y,
                },
                lower_right: Point {
                    x: bounds.lower_right.x,
                    y: mid_point.y,
                },
            },
            Quadrant::SouthEast => Bounds {
                upper_left: mid_point.clone(),
                lower_right: bounds.lower_right.clone(),
            },
        }
    }

    fn intersects_circle(&self, center: &Position, radius: f32) -> bool {
        let test_x = if center.x > self.upper_left.x {
            self.upper_left.x
        } else if center.x < self.lower_right.x {
            self.lower_right.x
        } else {
            center.x // If the circle is inside the bounds, measure distance to itself to always
                     // return true
        };

        let test_y = if center.y > self.upper_left.y {
            self.upper_left.y
        } else if center.y < self.lower_right.y {
            self.lower_right.y
        } else {
            center.y // If the circle is inside the bounds, measure distance to itself to always
                     // return true
        };

        let dist_x = center.x - test_x;
        let dist_x_sq = dist_x * dist_x;
        let dist_y = center.y - test_y;
        let dist_y_sq = dist_y * dist_y;

        (dist_x_sq + dist_y_sq) <= (radius * radius)
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::constants::HighGuidType;

    use super::*;

    fn insert(quadtree: &mut QuadTree, x: f32, y: f32, counter: u32) {
        let position = Position {
            x,
            y,
            z: 0.0,
            o: 0.0,
        };
        let guid = ObjectGuid::new(crate::shared::constants::HighGuidType::Player, counter);
        quadtree.insert(position, guid);
    }

    #[test]
    fn test_quadtree_insertion() {
        let mut quadtree = QuadTree::new(2);

        insert(&mut quadtree, 1.0, 1.0, 1);
        insert(&mut quadtree, 1.0, -1.0, 2);
        insert(&mut quadtree, -1.0, 1.0, 3);
        insert(&mut quadtree, -1.0, -1.0, 4);
        insert(&mut quadtree, -2.0, -2.0, 5);

        assert_eq!(
            format!("{quadtree:?}"),
            "QuadTree(2)<[NW[1],NE[2],SW[3],SE[4, 5]]>"
        );
    }

    #[test]
    fn test_deletion() {
        let mut quadtree = QuadTree::new(2);

        let pos1 = Position {
            x: 1.0,
            y: 1.0,
            z: 0.0,
            o: 0.0,
        };
        let guid1 = ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 1);
        let pos2 = Position {
            x: 1.0,
            y: -1.0,
            z: 0.0,
            o: 0.0,
        };
        let guid2 = ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 2);
        let pos3 = Position {
            x: -1.0,
            y: 1.0,
            z: 0.0,
            o: 0.0,
        };
        let guid3 = ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 3);
        let pos4 = Position {
            x: -1.0,
            y: -1.0,
            z: 0.0,
            o: 0.0,
        };
        let guid4 = ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 4);
        let pos5 = Position {
            x: -2.0,
            y: -2.0,
            z: 0.0,
            o: 0.0,
        };
        let guid5 = ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 5);

        insert(&mut quadtree, pos1.x, pos1.y, guid1.counter());
        insert(&mut quadtree, pos2.x, pos2.y, guid2.counter());
        insert(&mut quadtree, pos3.x, pos3.y, guid3.counter());
        insert(&mut quadtree, pos4.x, pos4.y, guid4.counter());
        insert(&mut quadtree, pos5.x, pos5.y, guid5.counter());

        quadtree.delete(&ObjectGuid::new(HighGuidType::Player, 4));
        assert_eq!(
            format!("{quadtree:?}"),
            "QuadTree(2)<[NW[1],NE[2],SW[3],SE[5]]>"
        );

        quadtree.delete(&ObjectGuid::new(HighGuidType::Player, 1));
        assert_eq!(
            format!("{quadtree:?}"),
            "QuadTree(2)<[NW[_],NE[2],SW[3],SE[5]]>"
        );

        quadtree.delete(&ObjectGuid::new(HighGuidType::Player, 2));
        quadtree.delete(&ObjectGuid::new(HighGuidType::Player, 3));
        quadtree.delete(&ObjectGuid::new(HighGuidType::Player, 5));
        assert_eq!(format!("{quadtree:?}"), "QuadTree(2)<[_]>");
    }

    #[test]
    fn test_intersection() {
        let bounds = Bounds {
            upper_left: Point { x: 0.0, y: 0.0 },
            lower_right: Point { x: -2.0, y: -2.0 },
        };

        // Circle is entirely within the rectangle
        let intersects = bounds.intersects_circle(
            &Position {
                x: -1.0,
                y: -1.0,
                z: 0.0,
                o: 0.0,
            },
            0.5,
        );
        assert_eq!(intersects, true);

        // Circle center is within but the circle itself is larger than the rectangle
        let intersects = bounds.intersects_circle(
            &Position {
                x: -1.0,
                y: -1.0,
                z: 0.0,
                o: 0.0,
            },
            5.0,
        );
        assert_eq!(intersects, true);

        // Circle center is West of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -1.0,
                y: 1.0,
                z: 0.0,
                o: 0.0,
            },
            0.5,
        );
        assert_eq!(intersects, false);

        // Circle center is West of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -1.0,
                y: 1.0,
                z: 0.0,
                o: 0.0,
            },
            1.1,
        );
        assert_eq!(intersects, true);

        // Circle center is North of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: 1.0,
                y: -1.0,
                z: 0.0,
                o: 0.0,
            },
            0.5,
        );
        assert_eq!(intersects, false);

        // Circle center is North of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: 1.0,
                y: -1.0,
                z: 0.0,
                o: 0.0,
            },
            1.1,
        );
        assert_eq!(intersects, true);

        // Circle center is East of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -1.0,
                y: -3.0,
                z: 0.0,
                o: 0.0,
            },
            0.5,
        );
        assert_eq!(intersects, false);

        // Circle center is East of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -1.0,
                y: -3.0,
                z: 0.0,
                o: 0.0,
            },
            1.1,
        );
        assert_eq!(intersects, true);

        // Circle center is South of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -3.0,
                y: -1.0,
                z: 0.0,
                o: 0.0,
            },
            0.5,
        );
        assert_eq!(intersects, false);

        // Circle center is South of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -3.0,
                y: -1.0,
                z: 0.0,
                o: 0.0,
            },
            1.1,
        );
        assert_eq!(intersects, true);

        // Circle center is North-West of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: 1.0,
                y: 1.0,
                z: 0.0,
                o: 0.0,
            },
            1.41,
        );
        assert_eq!(intersects, false);

        // Circle center is North-West of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: 1.0,
                y: 1.0,
                z: 0.0,
                o: 0.0,
            },
            1.42,
        );
        assert_eq!(intersects, true);

        // Circle center is North-East of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: 1.0,
                y: -3.0,
                z: 0.0,
                o: 0.0,
            },
            1.41,
        );
        assert_eq!(intersects, false);

        // Circle center is North-East of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: 1.0,
                y: -3.0,
                z: 0.0,
                o: 0.0,
            },
            1.42,
        );
        assert_eq!(intersects, true);

        // Circle center is South-East of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -3.0,
                y: -3.0,
                z: 0.0,
                o: 0.0,
            },
            1.41,
        );
        assert_eq!(intersects, false);

        // Circle center is South-East of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -3.0,
                y: -3.0,
                z: 0.0,
                o: 0.0,
            },
            1.42,
        );
        assert_eq!(intersects, true);

        // Circle center is South-West of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -3.0,
                y: 1.0,
                z: 0.0,
                o: 0.0,
            },
            1.41,
        );
        assert_eq!(intersects, false);

        // Circle center is South-West of the rectangle and overlapping
        let intersects = bounds.intersects_circle(
            &Position {
                x: -3.0,
                y: 1.0,
                z: 0.0,
                o: 0.0,
            },
            1.42,
        );
        assert_eq!(intersects, true);
    }

    #[test]
    fn test_quadtree_find() {
        fn find_sorted(quadtree: &QuadTree, position: &Position, radius: f32) -> Vec<u64> {
            let mut guids = quadtree
                .search_around_position(position, radius, false)
                .into_iter()
                .map(|g| g.raw())
                .collect::<Vec<u64>>();
            guids.sort();
            guids
        }

        fn build_pos(x: f32, y: f32) -> Position {
            Position {
                x,
                y,
                z: 0.0,
                o: 0.0,
            }
        }

        let mut quadtree = QuadTree::new(2);

        insert(&mut quadtree, 2.0, 2.0, 1);
        insert(&mut quadtree, 2.0, -2.0, 2);
        insert(&mut quadtree, -2.0, -2.0, 3);
        insert(&mut quadtree, -2.0, 2.0, 4);

        assert_eq!(
            find_sorted(&quadtree, &build_pos(0.0, 0.0), 4.0),
            vec![1, 2, 3, 4]
        );
        assert_eq!(
            find_sorted(&quadtree, &build_pos(0.0, 0.0), 2.0),
            Vec::<u64>::new()
        );
        assert_eq!(
            find_sorted(&quadtree, &build_pos(2.0, 0.0), 2.0),
            vec![1, 2]
        );
        assert_eq!(find_sorted(&quadtree, &build_pos(2.0, -2.0), 3.9), vec![2]);
        assert_eq!(
            find_sorted(&quadtree, &build_pos(2.0, -2.0), 4.0),
            vec![1, 2, 3]
        );
        assert_eq!(find_sorted(&quadtree, &build_pos(2.0, 2.0), 0.0), vec![1]);
    }
}
