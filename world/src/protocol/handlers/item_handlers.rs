use binrw::NullString;
use log::{error, warn};
use shipyard::{Get, View, ViewMut};

use crate::ecs::components::guid::Guid;
use crate::ecs::components::powers::Powers;
use crate::ecs::components::spell_cast::SpellCast;
use crate::entities::attributes::Attributes;
use crate::entities::player::Player;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::session::world_session::WSRunnableArgs;
use crate::shared::constants::SpellFailReason;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_item_query_single(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_item_query_single: CmsgItemQuerySingle = ClientMessage::read_as(data).unwrap();

        let packet = if let Some(item) = world_context
            .data_store
            .get_item_template(cmsg_item_query_single.item_id)
        {
            ServerMessage::new(SmsgItemQuerySingleResponse {
                result: None,
                template: Some(ItemQueryResponse {
                    item_id: item.entry,
                    item_class: item.class,
                    item_subclass: item.subclass,
                    item_unk: -1,
                    name: item.name.clone().into(),
                    name2: 0,
                    name3: 0,
                    name4: 0,
                    display_id: item.display_id,
                    quality: item.quality,
                    flags: item.flags,
                    buy_price: item.buy_price,
                    sell_price: item.sell_price,
                    inventory_type: item.inventory_type,
                    allowable_class: item.allowable_class,
                    allowable_race: item.allowable_race,
                    item_level: item.item_level,
                    required_level: item.required_level,
                    required_skill: item.required_skill,
                    required_skill_rank: item.required_skill,
                    required_spell: item.required_spell,
                    required_honor_rank: item.required_honor_rank,
                    required_city_rank: item.required_city_rank,
                    required_reputation_faction: item.required_reputation_faction,
                    required_reputation_rank: item.required_reputation_rank,
                    max_count: item.max_count,
                    max_stack_count: item.max_stack_count,
                    container_slots: item.container_slots,
                    stats: &item.stats,
                    damages: &item.damages,
                    armor: item.armor,
                    resist_holy: item.holy_res,
                    resist_fire: item.fire_res,
                    resist_nature: item.nature_res,
                    resist_frost: item.frost_res,
                    resist_shadow: item.shadow_res,
                    resist_arcane: item.arcane_res,
                    delay: item.delay,
                    ammo_type: item.ammo_type,
                    ranged_mod_range: item.ranged_mod_range,
                    spells: &item.spells,
                    bonding: item.bonding,
                    description: item.description.clone().into(),
                    page_text: item.page_text,
                    language_id: item.language_id,
                    page_material: item.page_material,
                    start_quest: item.start_quest,
                    lock_id: item.lock_id,
                    material: item.material,
                    sheath: item.sheath,
                    random_property: item.random_property,
                    random_suffix: item.random_suffix,
                    block: item.block,
                    item_set: item.itemset,
                    max_durability: item.max_durability,
                    area: item.area,
                    map: item.map,
                    bag_family: item.bag_family,
                    totem_category: item.totem_category,
                    sockets: &item.sockets,
                    socket_bonus: item.socket_bonus,
                    gem_properties: item.gem_properties,
                    required_enchantment_skill: item.required_disenchant_skill,
                    armor_damage_modifier: item.armor_damage_modifier,
                    duration: item.duration,
                }),
            })
        } else {
            ServerMessage::new(SmsgItemQuerySingleResponse {
                result: Some(cmsg_item_query_single.item_id | 0x80000000),
                template: None,
            })
        };

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_item_name_query(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_item_name_query: CmsgItemNameQuery = ClientMessage::read_as(data).unwrap();

        if let Some(item) = world_context
            .data_store
            .get_item_template(cmsg_item_name_query.item_id)
        {
            let packet = ServerMessage::new(SmsgItemNameQueryResponse {
                item_id: item.entry,
                name: NullString::from(item.name.clone()),
                inventory_type: item.inventory_type,
            });

            session.send(&packet).unwrap();
        }
    }

    pub(crate) fn handle_cmsg_destroy_item(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg_destroy_item: CmsgDestroyItem = ClientMessage::read_as(data).unwrap();

        if let Some(map) = session.current_map() {
            map.world().run(
                |mut vm_player: ViewMut<Player>,
                 mut vm_attributes: ViewMut<Attributes>| {
                    let player_entity_id = session.player_entity_id().unwrap();
                    let player = &mut vm_player[player_entity_id];
                    let attributes = &mut vm_attributes[player_entity_id];

                    if let Some(removed_item) =
                        player.remove_item(cmsg_destroy_item.slot.into(), attributes)
                    {
                        session.destroy_entity(removed_item.guid());
                    }
                },
            );
        }
    }

    pub(crate) fn handle_cmsg_auto_equip_item(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_auto_equip_item: CmsgAutoEquipItem = ClientMessage::read_as(data).unwrap();
        let slot = cmsg_auto_equip_item.slot as u32;

        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                          ..
                      }| {
            map.world().run(
                |mut vm_player: ViewMut<Player>,
                 mut vm_attributes: ViewMut<Attributes>| {
                    if let Ok(mut player) = (&mut vm_player).get(player_entity_id) {
                        let attributes = &mut vm_attributes[player_entity_id];

                        let inventory_result =
                            player.try_equip_item_from_inventory(slot, attributes);
                        if let Some(moved_item) = player.get_inventory_item(slot) {
                            let moved_item_template = world_context
                                .data_store
                                .get_item_template(moved_item.entry());
                            let packet = ServerMessage::new(SmsgInventoryChangeFailure::build(
                                inventory_result,
                                Some(moved_item.guid()).copied(),
                                moved_item_template,
                                None,
                            ));
                            session.send(&packet).unwrap();
                        }
                    }
                },
            );
        });
    }

    pub(crate) fn handle_cmsg_swap_inv_item(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_swap_inv_item: CmsgSwapInvItem = ClientMessage::read_as(data).unwrap();

        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                          ..
                      }| {
            map.world().run(
                |mut vm_player: ViewMut<Player>,
                 mut vm_attributes: ViewMut<Attributes>| {
                    if let Ok(mut player) = (&mut vm_player).get(player_entity_id) {
                        let attributes = &mut vm_attributes[player_entity_id];

                        let inventory_result = player.try_swap_inventory_item(
                            cmsg_swap_inv_item.from_slot.into(),
                            cmsg_swap_inv_item.to_slot.into(),
                            attributes,
                        );

                        let maybe_moved_item =
                            player.get_inventory_item(cmsg_swap_inv_item.from_slot.into());
                        let maybe_target_item =
                            player.get_inventory_item(cmsg_swap_inv_item.to_slot.into());

                        let moved_item_template = maybe_moved_item.and_then(|moved_item| {
                            world_context
                                .data_store
                                .get_item_template(moved_item.entry())
                        });

                        let packet = ServerMessage::new(SmsgInventoryChangeFailure::build(
                            inventory_result,
                            maybe_moved_item.map(|item| item.guid()).copied(),
                            moved_item_template,
                            maybe_target_item.map(|item| item.guid()).copied(),
                        ));
                        session.send(&packet).unwrap();
                    }
                },
            );
        });
    }

    pub(crate) fn handle_cmsg_split_item(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_split_item: CmsgSplitItem = ClientMessage::read_as(data).unwrap();

        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                          ..
                      }| {
            map.world().run(
                |mut vm_player: ViewMut<Player>,
                 mut vm_attributes: ViewMut<Attributes>| {
                    if let Ok(mut player) = (&mut vm_player).get(player_entity_id) {
                        let attributes = &mut vm_attributes[player_entity_id];

                        let inventory_result = player.try_split_item(
                            cmsg_split_item.source_slot.into(),
                            cmsg_split_item.destination_slot.into(),
                            cmsg_split_item.count,
                            attributes,
                        );

                        let maybe_source_item =
                            player.get_inventory_item(cmsg_split_item.source_slot.into());
                        let maybe_new_item =
                            player.get_inventory_item(cmsg_split_item.destination_slot.into());

                        let source_item_template = maybe_source_item.and_then(|moved_item| {
                            world_context
                                .data_store
                                .get_item_template(moved_item.entry())
                        });

                        let packet = ServerMessage::new(SmsgInventoryChangeFailure::build(
                            inventory_result,
                            maybe_source_item.map(|item| item.guid()).copied(),
                            source_item_template,
                            maybe_new_item.map(|item| item.guid()).copied(),
                        ));
                        session.send(&packet).unwrap();
                    }
                },
            );
        });
    }

    pub(crate) fn handle_cmsg_use_item(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgUseItem = ClientMessage::read_as(data).unwrap();

        // FIXME: This is duplicated from handle_cmsg_cast_spell
        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                          ..
                      }| {
            let mut targets = cmsg.targets.clone();
            targets.update_internal_refs(map.clone(), session.player_guid().unwrap());

            map.world().run(
                |v_player: View<Player>,
                 mut vm_spell: ViewMut<SpellCast>,
                 v_powers: View<Powers>,
                 v_guid: View<Guid>| {
                    let Ok(player) = v_player.get(player_entity_id) else {
                        error!("handle_cmsg_use_item: no player found");
                        return;
                    };

                    let Some(item) = player.inventory().get(cmsg.bag_slot.into()) else {
                        warn!("implement item not found in handle_cmsg_use_item");
                        return;
                    };

                    let Some(item_template) =
                        world_context.data_store.get_item_template(item.entry())
                    else {
                        error!("handle_cmsg_use_item: item template not found");
                        return;
                    };

                    // TODO: We actually need to cast all spells in the template
                    let spell_id = item_template.spells[0].id;

                    if vm_spell[player_entity_id].current_ranged().is_some() {
                        let packet = ServerMessage::new(SmsgCastFailed {
                            spell_id,
                            result: SpellFailReason::SpellInProgress,
                            cast_count: cmsg.cast_count,
                        });

                        session.send(&packet).unwrap();

                        return;
                    }

                    // TODO: all of this seems to be duplicated from SpellCast::cast_spell
                    let Some(spell_record) = world_context.data_store.get_spell_record(spell_id)
                    else {
                        warn!("attempt to cast non-existing spell {}", spell_id);
                        return;
                    };

                    let Some(spell_base_cast_time) =
                        spell_record.base_cast_time(world_context.data_store.clone())
                    else {
                        error!("spell {} has no base cast time in DBC", spell_id);
                        return;
                    };

                    let powers = &v_powers[player_entity_id];
                    let power_cost = spell_record.calculate_power_cost(
                        powers.base_health(),
                        powers.base_mana(),
                        powers.snapshot(),
                    );

                    let unit_target = targets.unit_target();
                    let unit_target_guid = unit_target
                        .and_then(|entity_id| v_guid.get(entity_id).ok())
                        .map(|g| g.0);

                    vm_spell[player_entity_id].set_current_ranged(
                        spell_id,
                        Some(item_template.entry),
                        spell_base_cast_time,
                        player_entity_id,
                        session.player_guid().unwrap(),
                        unit_target,
                        unit_target_guid,
                        targets.game_object_target(),
                        power_cost,
                    );

                    let packet = ServerMessage::new(SmsgClearExtraAuraInfo {
                        target_guid: session.player_guid().unwrap().as_packed(),
                        spell_id,
                    });

                    session.send(&packet).unwrap();

                    let packet = ServerMessage::new(SmsgSpellStart {
                        caster_entity_guid: session.player_guid().unwrap().as_packed(),
                        caster_unit_guid: session.player_guid().unwrap().as_packed(),
                        spell_id,
                        cast_id: cmsg.cast_count,
                        cast_flags: 0,
                        cast_time: spell_base_cast_time,
                        target_flags: 0,
                    });

                    session.send(&packet).unwrap();
                },
            );
        });
    }
}
