use log::{error, warn};
use shipyard::{Get, View, ViewMut};

use crate::{
    datastore::data_types::GameObjectData,
    ecs::components::{movement::Movement, unit::Unit},
    entities::{
        game_object::GameObject,
        player::{player_data::BindPoint, Player},
        position::WorldPosition,
    },
    game::{
        map_manager::MapKey,
        spell_effect_handler::{SpellEffectHandler, SpellEffectHandlerArgs},
    },
    protocol::{
        packets::{LootResponseItem, SmsgBindpointUpdate, SmsgLootResponse, SmsgPlayerBound},
        server::ServerMessage,
    },
    shared::constants::{LootSlotType, LootType, SpellTargetType, UnitFlags},
};

impl SpellEffectHandler {
    pub fn handle_effect_open_lock(
        SpellEffectHandlerArgs {
            world_context,
            spell,
            all_storages,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        all_storages.run(
            |v_game_object: View<GameObject>,
             v_unit: View<Unit>,
             mut vm_player: ViewMut<Player>| {
                let Some(game_object_target) = spell.game_object_target() else {
                    warn!("spell effect OpenLock: no game object target");
                    return;
                };

                if let Ok(game_object) = v_game_object.get(game_object_target) {
                    let player = &mut vm_player[spell.caster()];
                    // TODO: Check that the player can open this lock (CanOpenLock in MaNGOS)

                    v_unit[spell.caster()].set_unit_flag(UnitFlags::Looting);
                    player.set_looting(spell.game_object_target());

                    game_object.generate_loot(false);

                    let loot_items: Vec<LootResponseItem> = game_object
                        .loot()
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
                        &game_object.guid(),
                        LootType::Pickpocketing,
                        0,
                        loot_items,
                    ));
                    player.session.send(&packet).unwrap();

                    // Type-specific handling
                    #[allow(clippy::single_match)] // More types to come later
                    match game_object.data {
                        GameObjectData::Goober { .. } => player.notify_interacted_with_game_object(
                            &game_object.guid(),
                            game_object.entry,
                        ),
                        _ => (),
                    }

                    // TODO: Increase this lock's skill (end of EffectOpenLock in MaNGOS)
                }
            },
        );
    }

    pub fn handle_effect_bind(
        SpellEffectHandlerArgs {
            spell,
            all_storages,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        let Some(unit_target_entity_id) = spell.unit_target() else {
            warn!("handle_effect_bind: spell has no unit target");
            return;
        };

        all_storages.run(|vm_player: ViewMut<Player>, v_wpos: View<WorldPosition>| {
            let Ok(player) = &mut vm_player.get(unit_target_entity_id) else {
                warn!("handle_effect_bind: spell unit target is not a player");
                return;
            };

            let Ok(player_position) = v_wpos.get(unit_target_entity_id) else {
                warn!("handle_effect_bind: player has no position");
                return;
            };

            let area_id = player
                .session
                .current_map()
                .unwrap()
                .get_area_id(player_position.x, player_position.y)
                .unwrap_or(0);

            let bindpoint = &BindPoint::from_position(player_position, area_id);

            player.set_bindpoint(*bindpoint);

            let packet = ServerMessage::new(SmsgBindpointUpdate::from_bindpoint(bindpoint));

            player.session.send(&packet).unwrap();

            let packet = ServerMessage::new(SmsgPlayerBound {
                caster_guid: spell.caster_guid(),
                area_id,
            });

            player.session.send(&packet).unwrap();
        })
    }

    pub fn handle_effect_teleport_units(
        SpellEffectHandlerArgs {
            spell,
            all_storages,
            spell_record,
            effect_index,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        let target_type = match spell_record.effect_implicit_target_b[effect_index] {
            SpellTargetType::None => spell_record.effect_implicit_target_a[effect_index],
            other => other,
        };

        match target_type {
            SpellTargetType::InnkeeperCoordinates => {
                let vm_player = &mut all_storages.borrow::<ViewMut<Player>>().unwrap();
                let Some(target_entity_id) = spell.unit_target() else {
                    error!("handle_effect_teleport_units: spell has no unit target");
                    return;
                };

                let Ok(player) = &mut vm_player.get(target_entity_id) else {
                    error!("handle_effect_teleport_units: unit target is not a player");
                    return;
                };

                let bindpoint = player.bindpoint();
                let destination = WorldPosition {
                    map_key: MapKey::for_continent(bindpoint.map_id),
                    zone: 0, // TODO: get zone from terrain files
                    x: bindpoint.x,
                    y: bindpoint.y,
                    z: bindpoint.z,
                    o: bindpoint.o,
                };

                let v_wpos = all_storages.borrow::<View<WorldPosition>>().unwrap();
                let v_movement = all_storages.borrow::<View<Movement>>().unwrap();
                player.teleport_to(&destination, false, v_wpos, v_movement);
            }
            _ => warn!(
                "handle_effect_teleport_units: target_type {target_type:?} not implemented yet"
            ),
        }
    }
}
