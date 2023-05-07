use shared::models::terrain_info::MAP_MAX_COORD;

use crate::entities::{object_guid::ObjectGuid, position::Position};

// QuadTree to store entities position in the world
#[derive(Debug, Clone)]
struct GuidPosition {
    // TODO: use refs?
    pub guid: ObjectGuid,
    pub pos: Position,
}

impl GuidPosition {
    pub fn point(&self) -> Point {
        Point {
            x: self.pos.x,
            y: self.pos.y,
        }
    }
}

#[derive(Debug)]
enum NodeContent {
    Empty,
    Values(Vec<GuidPosition>),
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
    fn select(coords: &Point, bounds: &Bounds) -> Self {
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
            NodeContent::Values(vs) => write!(
                f,
                "{:?}",
                vs.iter().map(|v| v.guid.raw()).collect::<Vec<u64>>()
            ),
            NodeContent::Children { nw, ne, sw, se } => {
                write!(f, "[NW{:?},NE{:?},SW{:?},SE{:?}]", nw, ne, sw, se)
            }
        }
    }
}

impl Node {
    pub fn leaf(value: GuidPosition, bounds: Bounds) -> Node {
        Node {
            content: NodeContent::Values(vec![value]),
            bounds,
        }
    }

    pub fn leaf_or_empty(values: Vec<GuidPosition>, bounds: Bounds) -> Node {
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
        }
    }

    pub fn insert(&mut self, pos: Position, guid: ObjectGuid) {
        fn insert_rec(node: &mut Box<Node>, new_value: GuidPosition, node_capacity: usize) {
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
                    let mut nw: Vec<GuidPosition> = Vec::new();
                    let mut ne: Vec<GuidPosition> = Vec::new();
                    let mut sw: Vec<GuidPosition> = Vec::new();
                    let mut se: Vec<GuidPosition> = Vec::new();

                    for value in existing_values {
                        match Quadrant::select(&value.point(), &node.bounds) {
                            Quadrant::NorthWest => nw.push(value.clone()),
                            Quadrant::NorthEast => ne.push(value.clone()),
                            Quadrant::SouthWest => sw.push(value.clone()),
                            Quadrant::SouthEast => se.push(value.clone()),
                        }
                    }

                    // ...then insert in the relevant quadrant
                    match Quadrant::select(&new_value.point(), &node.bounds) {
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
                    let new_value_quadrant = Quadrant::select(&new_value.point(), &node.bounds);

                    let target_quadrant = match new_value_quadrant {
                        Quadrant::NorthWest => nw,
                        Quadrant::NorthEast => ne,
                        Quadrant::SouthWest => sw,
                        Quadrant::SouthEast => se,
                    };

                    insert_rec(target_quadrant, new_value, node_capacity);
                }
            }
        }

        let guidpos = GuidPosition { guid, pos };

        insert_rec(&mut self.root, guidpos, self.node_capacity);
    }

    pub fn search(&self, center_x: f32, center_y: f32, radius: f32) -> Vec<ObjectGuid> {
        fn search_rec(node: &Box<Node>, center: &Point, radius: f32, acc: &mut Vec<ObjectGuid>) {
            match &node.content {
                NodeContent::Empty => (),
                NodeContent::Values(values) if node.bounds.intersects_circle(&center, radius) => {
                    let radius_square = radius * radius;
                    for value in values {
                        let dist_square = value.point().dist_square(&center);
                        if dist_square <= radius_square {
                            acc.push(value.guid.clone());
                        }
                    }
                }
                NodeContent::Values(_) => (),
                NodeContent::Children { nw, ne, sw, se } => {
                    if nw.bounds.intersects_circle(&center, radius) {
                        search_rec(nw, &center, radius, acc);
                    }
                    if ne.bounds.intersects_circle(&center, radius) {
                        search_rec(ne, &center, radius, acc);
                    }
                    if sw.bounds.intersects_circle(&center, radius) {
                        search_rec(sw, &center, radius, acc);
                    }
                    if se.bounds.intersects_circle(&center, radius) {
                        search_rec(se, &center, radius, acc);
                    }
                }
            }
        }

        let center = Point {
            x: center_x,
            y: center_y,
        };
        let mut guids: Vec<ObjectGuid> = Vec::new();
        search_rec(&self.root, &center, radius, &mut guids);
        guids
    }

    pub fn delete(&mut self, pos: Position, guid: ObjectGuid) {
        fn delete_rec(node: &mut Box<Node>, value: &GuidPosition) {
            match &mut (*node).content {
                NodeContent::Values(ref mut existing_values) => {
                    existing_values.retain(|v| v.guid != value.guid);

                    if existing_values.is_empty() {
                        **node = Node::empty(node.bounds.clone());
                    }
                }
                NodeContent::Children { nw, ne, sw, se } => {
                    let value_quadrant = Quadrant::select(&value.point(), &node.bounds);

                    match value_quadrant {
                        Quadrant::NorthWest => delete_rec(nw, value),
                        Quadrant::NorthEast => delete_rec(ne, value),
                        Quadrant::SouthWest => delete_rec(sw, value),
                        Quadrant::SouthEast => delete_rec(se, value),
                    };

                    if node.is_empty() {
                        **node = Node::empty(node.bounds.clone());
                    }
                }
                _ => (),
            }
        }

        let guidpos = GuidPosition { guid, pos };

        delete_rec(&mut self.root, &guidpos)
    }

    pub fn update(&mut self, old_position: Position, new_position: Position, guid: ObjectGuid) {
        // Possible optimization: search for the value and update it in place if the new position
        // ends up in the same node as the old position
        // For now, simply delete then insert
        self.delete(old_position, guid);
        self.insert(new_position, guid);
    }
}

#[derive(Clone, Debug)]
struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn dist_square(&self, other: &Point) -> f32 {
        let dist_x = self.x - other.x;
        let dist_y = self.y - other.y;

        (dist_x * dist_x) + (dist_y * dist_y)
    }
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

    fn intersects_circle(&self, center: &Point, radius: f32) -> bool {
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

        quadtree.delete(
            pos4,
            ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 4),
        );
        assert_eq!(
            format!("{quadtree:?}"),
            "QuadTree(2)<[NW[1],NE[2],SW[3],SE[5]]>"
        );

        quadtree.delete(
            pos1,
            ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 1),
        );
        assert_eq!(
            format!("{quadtree:?}"),
            "QuadTree(2)<[NW[_],NE[2],SW[3],SE[5]]>"
        );

        quadtree.delete(
            pos2,
            ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 2),
        );
        quadtree.delete(
            pos3,
            ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 3),
        );
        quadtree.delete(
            pos5,
            ObjectGuid::new(crate::shared::constants::HighGuidType::Player, 5),
        );
        assert_eq!(format!("{quadtree:?}"), "QuadTree(2)<[_]>");
    }

    #[test]
    fn test_intersection() {
        let bounds = Bounds {
            upper_left: Point { x: 0.0, y: 0.0 },
            lower_right: Point { x: -2.0, y: -2.0 },
        };

        // Circle is entirely within the rectangle
        let intersects = bounds.intersects_circle(&Point { x: -1.0, y: -1.0 }, 0.5);
        assert_eq!(intersects, true);

        // Circle center is within but the circle itself is larger than the rectangle
        let intersects = bounds.intersects_circle(&Point { x: -1.0, y: -1.0 }, 5.0);
        assert_eq!(intersects, true);

        // Circle center is West of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: -1.0, y: 1.0 }, 0.5);
        assert_eq!(intersects, false);

        // Circle center is West of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: -1.0, y: 1.0 }, 1.1);
        assert_eq!(intersects, true);

        // Circle center is North of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: 1.0, y: -1.0 }, 0.5);
        assert_eq!(intersects, false);

        // Circle center is North of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: 1.0, y: -1.0 }, 1.1);
        assert_eq!(intersects, true);

        // Circle center is East of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: -1.0, y: -3.0 }, 0.5);
        assert_eq!(intersects, false);

        // Circle center is East of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: -1.0, y: -3.0 }, 1.1);
        assert_eq!(intersects, true);

        // Circle center is South of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: -3.0, y: -1.0 }, 0.5);
        assert_eq!(intersects, false);

        // Circle center is South of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: -3.0, y: -1.0 }, 1.1);
        assert_eq!(intersects, true);

        // Circle center is North-West of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: 1.0, y: 1.0 }, 1.41);
        assert_eq!(intersects, false);

        // Circle center is North-West of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: 1.0, y: 1.0 }, 1.42);
        assert_eq!(intersects, true);

        // Circle center is North-East of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: 1.0, y: -3.0 }, 1.41);
        assert_eq!(intersects, false);

        // Circle center is North-East of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: 1.0, y: -3.0 }, 1.42);
        assert_eq!(intersects, true);

        // Circle center is South-East of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: -3.0, y: -3.0 }, 1.41);
        assert_eq!(intersects, false);

        // Circle center is South-East of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: -3.0, y: -3.0 }, 1.42);
        assert_eq!(intersects, true);

        // Circle center is South-West of the rectangle and not overlapping
        let intersects = bounds.intersects_circle(&Point { x: -3.0, y: 1.0 }, 1.41);
        assert_eq!(intersects, false);

        // Circle center is South-West of the rectangle and overlapping
        let intersects = bounds.intersects_circle(&Point { x: -3.0, y: 1.0 }, 1.42);
        assert_eq!(intersects, true);
    }

    #[test]
    fn test_quadtree_find() {
        fn find_sorted(quadtree: &QuadTree, center_x: f32, center_y: f32, radius: f32) -> Vec<u64> {
            let mut guids = quadtree
                .search(center_x, center_y, radius)
                .into_iter()
                .map(|g| g.raw())
                .collect::<Vec<u64>>();
            guids.sort();
            guids
        }

        let mut quadtree = QuadTree::new(2);

        insert(&mut quadtree, 2.0, 2.0, 1);
        insert(&mut quadtree, 2.0, -2.0, 2);
        insert(&mut quadtree, -2.0, -2.0, 3);
        insert(&mut quadtree, -2.0, 2.0, 4);

        assert_eq!(find_sorted(&quadtree, 0.0, 0.0, 4.0), vec![1, 2, 3, 4]);
        assert_eq!(find_sorted(&quadtree, 0.0, 0.0, 2.0), Vec::<u64>::new());
        assert_eq!(find_sorted(&quadtree, 2.0, 0.0, 2.0), vec![1, 2]);
        assert_eq!(find_sorted(&quadtree, 2.0, -2.0, 3.9), vec![2]);
        assert_eq!(find_sorted(&quadtree, 2.0, -2.0, 4.0), vec![1, 2, 3]);
        assert_eq!(find_sorted(&quadtree, 2.0, 2.0, 0.0), vec![1]);
    }
}
