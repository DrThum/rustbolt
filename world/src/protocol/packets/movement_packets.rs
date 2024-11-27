use binrw::{binread, binwrite};
use enumflags2::BitFlags;
use opcode_derive::server_opcode;
use shared::models::terrain_info::Vector3;

use crate::entities::object_guid::{ObjectGuid, PackedObjectGuid};
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::SplineFlag;

use super::MovementInfo;

#[binwrite]
#[server_opcode]
pub struct SmsgMoveSetCanFly {
    pub guid: PackedObjectGuid,
    pub counter: u32,
}

impl SmsgMoveSetCanFly {
    pub fn build(guid: &ObjectGuid) -> Self {
        Self {
            guid: guid.as_packed(),
            counter: 0,
        } // TODO: Implement ACK etc
    }
}

#[binwrite]
#[server_opcode]
pub struct SmsgMoveUnsetCanFly {
    pub guid: PackedObjectGuid,
    pub counter: u32,
}

impl SmsgMoveUnsetCanFly {
    pub fn build(guid: &ObjectGuid) -> Self {
        Self {
            guid: guid.as_packed(),
            counter: 0,
        } // TODO: Implement ACK etc
    }
}

#[binwrite]
#[server_opcode]
pub struct MsgMoveTeleportAck {
    pub packed_guid: PackedObjectGuid,
    pub unk_counter: u32,
    pub movement_info: MovementInfo,
}

#[binread]
pub struct MsgMoveTeleportAckFromClient {
    pub _guid: ObjectGuid,
    pub _counter: u32,
    pub _time: u32,
}

// https://gist.github.com/LordJZ/1355974#file-monstermove-cs-L118
#[binwrite]
#[server_opcode]
pub struct SmsgMonsterMove {
    guid: PackedObjectGuid,
    current_position: Vector3,
    tick_count: u32,
    move_type: u8, // See MonsterMoveType in mangos one
    #[bw(map = |bf: &BitFlags<SplineFlag>| bf.bits())]
    spline_flags: BitFlags<SplineFlag>,
    duration: u32,
    point_count: u32,
    #[bw(if(!spline_flags.contains(SplineFlag::Flying)))]
    linear_path: Option<MonsterMoveLinearPath>,
    #[bw(if(spline_flags.contains(SplineFlag::Flying)))]
    catmullrom_path: Option<MonsterMoveCatmullRomPath>,
}

#[binwrite]
pub struct MonsterMoveLinearPath {
    destination: Vector3,
    points: Vec<u32>,
}

#[binwrite]
pub struct MonsterMoveCatmullRomPath {
    points: Vec<Vector3>,
}

impl SmsgMonsterMove {
    pub fn build(
        monster_guid: &ObjectGuid,
        current_position: &Vector3,
        path: Vec<Vector3>,
        spline_id: u32,
        move_type: u8,
        spline_flags: BitFlags<SplineFlag>,
        duration: u32,
    ) -> Self {
        assert!(
            !path.is_empty(),
            "cannot build a SmsgMonsterMove packet with an empty path"
        );

        let mut point_count = path.len() as u32;
        let mut linear_path: Option<MonsterMoveLinearPath> = None;
        let mut catmullrom_path: Option<MonsterMoveCatmullRomPath> = None;

        if spline_flags.contains(SplineFlag::Flying) {
            if spline_flags.contains(SplineFlag::Cyclic) {
                point_count += 1;
                // Send the first point twice, it will be erased by the client
                // after the first cycle
                let mut points = vec![*path.first().unwrap()];
                points.extend(path);

                catmullrom_path = Some(MonsterMoveCatmullRomPath { points });
            } else {
                catmullrom_path = Some(MonsterMoveCatmullRomPath { points: path });
            }
        } else {
            let mut path = path.clone();
            let destination = path.pop().unwrap();
            let packed_against = current_position.add(&destination).div(2.);

            let mut points: Vec<u32> = Vec::new();
            for point in path {
                let offset = packed_against - point;
                points.push(offset.pack())
            }
            linear_path = Some(MonsterMoveLinearPath {
                destination,
                points,
            });
        }

        Self {
            guid: monster_guid.as_packed(),
            current_position: *current_position,
            tick_count: spline_id,
            move_type,
            spline_flags,
            duration,
            point_count,
            linear_path,
            catmullrom_path,
        }
    }
}
