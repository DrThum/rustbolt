use std::sync::Arc;

use log::{error, warn};
use shipyard::{Get, View, ViewMut};

use crate::ecs::components::unit::Unit;
use crate::entities::creature::Creature;
use crate::entities::game_object::GameObject;
use crate::entities::object_guid::ObjectGuid;
use crate::entities::player::Player;
use crate::game::loot::Loot;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::{WSRunnableArgs, WorldSession};
use crate::shared::constants::{LootSlotType, LootType, UnitDynamicFlag, UnitFlags};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_loot(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
    ) {
        let cmsg: CmsgLoot = ClientMessage::read_as(data).unwrap();

        if let Some(target_guid) = ObjectGuid::from_raw(cmsg.target_guid) {
            if let Some(map) = session.current_map() {
                if let Some(looted_entity_id) = map.lookup_entity_ecs(&target_guid) {
                    let maybe_loot: Option<Loot> = map.world().run(
                        |v_creature: View<Creature>, v_game_object: View<GameObject>| {
                            if let Ok(creature) = v_creature.get(looted_entity_id) {
                                return Some(creature.loot());
                            }

                            if let Ok(game_object) = v_game_object.get(looted_entity_id) {
                                return Some(game_object.loot());
                            }

                            None
                        },
                    );

                    if let Some(loot) = maybe_loot {
                        map.world()
                            .run(|v_unit: View<Unit>, mut vm_player: ViewMut<Player>| {
                                v_unit[session.player_entity_id().unwrap()]
                                    .set_unit_flag(UnitFlags::Looting);

                                vm_player[session.player_entity_id().unwrap()]
                                    .set_looting(Some(looted_entity_id));
                            });

                        let loot_items = loot
                            .items()
                            .iter()
                            .map(|li| {
                                if let Some(item_template) =
                                    world_context.data_store.get_item_template(li.item_id)
                                {
                                    LootResponseItem {
                                        index: li.index,
                                        id: li.item_id,
                                        count: li.count,
                                        display_info_id: item_template.display_id,
                                        random_suffix: li.random_suffix,
                                        random_property_id: li.random_property_id,
                                        slot_type: LootSlotType::Normal,
                                    }
                                } else {
                                    panic!("found non-existing item when generating creature loot");
                                }
                            })
                            .collect();

                        let packet = ServerMessage::new(SmsgLootResponse::build(
                            &target_guid,
                            LootType::Corpse,
                            loot.money(),
                            loot_items,
                        ));
                        session.send(&packet).unwrap();
                    }
                }
            }
        }
    }

    pub(crate) fn handle_cmsg_loot_money(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        _data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
    ) {
        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                          ..
                      }| {
            map.world()
                .run(|v_player: View<Player>, v_creature: View<Creature>| {
                    let player = &v_player[player_entity_id];

                    if let Some(looted_entity_id) = player.currently_looting() {
                        if let Ok(creature) = v_creature.get(looted_entity_id) {
                            let loot_money = creature.remove_loot_money();
                            player.modify_money(loot_money as i32);

                            let packet =
                                ServerMessage::new(SmsgLootMoneyNotify { money: loot_money });
                            session.send(&packet).unwrap();

                            let packet = ServerMessage::new(SmsgLootClearMoney {});
                            session.send(&packet).unwrap();
                        }
                    } else {
                        warn!("received CMSG_LOOT_MONEY but player is not looting");
                    }
                });
        });
    }

    pub(crate) fn handle_cmsg_loot_release(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
    ) {
        let cmsg: CmsgLootRelease = ClientMessage::read_as(data).unwrap();

        if let Some(looted_guid) = ObjectGuid::from_raw(cmsg.looted_guid) {
            if let Some(map) = session.current_map() {
                if let Some(looted_entity_id) = map.lookup_entity_ecs(&looted_guid) {
                    map.world().run(|mut vm_player: ViewMut<Player>, v_unit: View<Unit>, v_creature: View<Creature>| {
                        let player_entity_id = session.player_entity_id().unwrap();
                        let player = &mut vm_player[player_entity_id];

                        match player.currently_looting() {
                            Some(player_looted_entity_id) if player_looted_entity_id != looted_entity_id => {
                                error!("received loot release for another entity than the one the player is currently looting");
                            },
                            None => error!("received loot release but player is not currently looting"),
                            _ => (),
                        }

                        player.set_looting(None);
                        v_unit[player_entity_id].unset_unit_flag(UnitFlags::Looting);

                        if let Ok(creature) = v_creature.get(looted_entity_id) {
                            if creature.loot().is_empty() {
                                if let Ok(unit) = v_unit.get(looted_entity_id) {
                                    unit.unset_dynamic_flag(UnitDynamicFlag::Lootable);
                                }
                            }
                        }
                    });

                    let packet =
                        ServerMessage::new(SmsgLootReleaseResponse::build(cmsg.looted_guid));
                    session.send(&packet).unwrap();
                }
            }
        }
    }

    pub(crate) fn handle_cmsg_autostore_loot_item(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
    ) {
        let cmsg: CmsgAutostoreLootItem = ClientMessage::read_as(data).unwrap();

        if let Some(map) = session.current_map() {
            map.world().run(
                |mut vm_player: ViewMut<Player>,
                 v_creature: View<Creature>,
                 v_game_object: View<GameObject>| {
                    let player_entity_id = session.player_entity_id().unwrap();
                    let player = &mut vm_player[player_entity_id];

                    if let Some(looted_entity_id) = player.currently_looting() {
                        let maybe_loot = {
                            if let Ok(creature) = v_creature.get(looted_entity_id) {
                                Some(creature.loot_mut())
                            } else if let Ok(game_object) = v_game_object.get(looted_entity_id) {
                                Some(game_object.loot_mut())
                            } else {
                                None
                            }
                        };

                        if let Some(mut loot) = maybe_loot {
                            if let Some(loot_item) = loot.get_item(cmsg.loot_index) {
                                match player.auto_store_new_item(loot_item.item_id, loot_item.count)
                                {
                                    Ok(_) => {
                                        loot.remove_item(cmsg.loot_index);

                                        let packet = ServerMessage::new(SmsgLootRemoved {
                                            loot_index: cmsg.loot_index,
                                        });
                                        session.send(&packet).unwrap();
                                    }
                                    Err(item_storage_error) => {
                                        let packet =
                                            ServerMessage::new(SmsgInventoryChangeFailure::build(
                                                item_storage_error,
                                                None,
                                                None,
                                                None,
                                            ));

                                        session.send(&packet).unwrap();
                                    }
                                }
                            }
                        }
                    }
                },
            );
        }
    }
}
