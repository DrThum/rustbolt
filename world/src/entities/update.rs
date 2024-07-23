use binrw::binwrite;
use enumflags2::{bitflags, BitFlags};
use fixedbitset::FixedBitSet;
use shared::models::terrain_info::Vector3;

use crate::{
    game::movement_spline::MovementSpline,
    shared::constants::{MovementFlag, ObjectTypeId},
};

use super::{object_guid::PackedObjectGuid, position::Position};

#[binwrite]
#[derive(Clone, Debug)]
pub struct CreateData {
    #[bw(map = |ut: &UpdateType| *ut as u8)]
    pub update_type: UpdateType,
    pub packed_guid: PackedObjectGuid,
    #[bw(map = |ot: &ObjectTypeId| *ot as u8)]
    pub object_type: ObjectTypeId,
    #[bw(map = |bf: &BitFlags<UpdateFlag>| bf.bits())]
    pub flags: BitFlags<UpdateFlag>,
    pub movement: Option<MovementUpdateData>, // Only if flags & Living
    pub position: Option<PositionUpdateData>, // Only if flags & HasPosition and !Living
    pub low_guid_part: Option<u32>,           // Only if flags & LowGuid
    pub high_guid_part: Option<u32>,          // Only if flags & HighGuid
    pub blocks: UpdateBlock,
}

#[binwrite]
pub struct UpdateData {
    #[bw(map = |ut: &UpdateType| *ut as u8)]
    pub update_type: UpdateType,
    pub packed_guid: PackedObjectGuid,
    pub blocks: UpdateBlock,
}

pub struct UpdateBlockBuilder {
    block_masks: Vec<FixedBitSet>,
    blocks: Vec<UpdateBlockValue>,
}

// Formatted update block for the client:
//
// * num_masks represent the number of block_masks to expect
// * block_masks contains bits whose index indicates which fields are being updated
// * data contains the data, one value for each bit set to 1 in the masks, in the same order
#[binwrite]
#[derive(Clone, Debug)]
pub struct UpdateBlock {
    pub num_masks: u8,
    pub block_masks: Vec<u32>,
    pub data: Vec<[u8; 4]>,
}

impl UpdateBlockBuilder {
    pub fn new() -> UpdateBlockBuilder {
        UpdateBlockBuilder {
            block_masks: vec![],
            blocks: vec![],
        }
    }

    pub fn add(&mut self, index: usize, value: u32) {
        let block_index = index / 32;
        let block_offset = index % 32;

        if block_index >= self.block_masks.len() {
            // Ensure that we have enough masks
            self.block_masks
                .resize(block_index + 1, FixedBitSet::with_capacity(32));
        }

        let block_bitset: &mut FixedBitSet = &mut self.block_masks[block_index];
        let was_already_set = block_bitset.put(block_offset);
        if was_already_set {
            // Remove the existing value first
            self.blocks.retain(|ub| ub.index != index);
        }

        self.blocks.push(UpdateBlockValue {
            index,
            value: value.to_le_bytes(),
        });
    }

    pub fn build(mut self) -> UpdateBlock {
        let block_masks: Vec<u32> = self
            .block_masks
            .iter()
            .map(|mask| *mask.as_slice().first().unwrap())
            .collect();

        self.blocks.sort_by_key(|b| b.index);
        let data: Vec<[u8; 4]> = self.blocks.into_iter().map(|b| b.value).collect();

        UpdateBlock {
            num_masks: self.block_masks.len() as u8,
            block_masks,
            data,
        }
    }
}

// Represent a value to be updated on the client for an entity.
//
// index: check [update_fields] for possible values
// value: all values are sent as 4 bytes, with padding if needed
struct UpdateBlockValue {
    pub index: usize,
    pub value: [u8; 4],
}

#[binwrite]
#[derive(Clone, Debug)]
pub struct MovementUpdateData {
    #[bw(map = |bf: &BitFlags<MovementFlag>| bf.bits())]
    pub movement_flags: BitFlags<MovementFlag>,
    pub movement_flags2: u8, // Always 0 in 2.4.3
    pub timestamp: u32,
    pub position: Position,
    pub pitch: Option<f32>,
    pub fall_time: u32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub speed_run_backward: f32,
    pub speed_swim: f32,
    pub speed_swim_backward: f32,
    pub speed_flight: f32,
    pub speed_flight_backward: f32,
    pub speed_turn: f32,
    #[bw(if(movement_flags.contains(MovementFlag::SplineEnabled)))]
    pub current_movement: Option<CurrentMovementData>,
}

#[binwrite]
#[derive(Clone, Debug)]
pub struct PositionUpdateData {
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
}

#[binwrite]
#[derive(Debug, Clone)]
pub struct CurrentMovementData {
    spline_flags: u32,
    ends_facing_angle: Option<f32>,
    ends_facing_target: Option<u64>,
    ends_facing_point: Option<Vector3>,
    elapsed_time: u32,
    total_time: u32,
    spline_id: u32,
    point_count: u32,
    path: Vec<Vector3>,
    destination: Vector3, // Vector3::ZERO if path is cyclic
}

impl CurrentMovementData {
    pub fn build(movement_flags: BitFlags<MovementFlag>, spline: &MovementSpline) -> Option<Self> {
        let spline_enabled = movement_flags.contains(MovementFlag::SplineEnabled);

        if spline_enabled {
            let path = spline.path().clone();
            let destination = *path.last().unwrap_or(&Vector3::ZERO);

            Some(Self {
                spline_flags: spline.spline_flags().bits(),
                ends_facing_angle: None,  // TODO
                ends_facing_target: None, // TODO
                ends_facing_point: None,  // TODO
                elapsed_time: spline.elapsed_time().as_millis() as u32,
                total_time: spline.total_time().as_millis() as u32,
                spline_id: spline.id(),
                point_count: path.len() as u32,
                path,
                destination,
            })
        } else {
            None
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum UpdateType {
    Values = 0,
    Movement = 1,
    CreateObject = 2,
    CreateObject2 = 3,
    OutOfRangeObjects = 4,
    NearObjects = 5,
}

#[allow(dead_code)]
#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum UpdateFlag {
    SelfUpdate = 0x01, // Self is a reserved keyword
    Transport = 0x02,
    HasAttackingTarget = 0x04,
    LowGuid = 0x08,
    HighGuid = 0x10,
    Living = 0x20,
    HasPosition = 0x40,
}
