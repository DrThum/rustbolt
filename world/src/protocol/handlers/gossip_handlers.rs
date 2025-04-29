use binrw::NullString;
use log::{error, warn};
use shipyard::{Get, View, ViewMut};

use crate::datastore::data_types::ItemTemplate;
use crate::entities::attributes::Attributes;
use crate::entities::creature::Creature;
use crate::entities::player::Player;
use crate::game::gossip::GossipMenu;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::shared::constants::{
    BuyFailedReason, CharacterClass, GossipMenuOptionType, SellFailedReason, TrainerType,
};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_gossip_hello(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgGossipHello = ClientMessage::read_as(data).unwrap();

        OpcodeHandler::send_initial_gossip_menu(cmsg.guid, session.clone(), world_context.clone());
    }

    pub fn handle_cmsg_gossip_select_option(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgGossipSelectOption = ClientMessage::read_as(data).unwrap();

        let Some(map) = session.current_map() else {
            error!("handle_cmsg_gossip_select_option: session has no map");
            return;
        };

        let Some(target_entity_id) = map.lookup_entity_ecs(&cmsg.guid) else {
            error!(
                "handle_cmsg_gossip_select_option: map has no EntityId for cmsg.guid (guid: {:?})",
                cmsg.guid
            );
            return;
        };

        let Ok(creature_template) = map.world().run(|v_creature: View<Creature>| {
            v_creature.get(target_entity_id).map(|c| c.template.clone())
        }) else {
            warn!("handle_cmsg_gossip_select_option: target is not a creature, TODO!");
            return;
        };

        let Some(gossip_menu_record) = world_context.data_store.get_gossip_menu(cmsg.menu_id)
        else {
            error!("handle_cmsg_gossip_select_option: received a non-existing menu_id");
            return;
        };

        let gossip_menu = GossipMenu::from_db_record(gossip_menu_record);

        if cmsg.option_index as usize >= gossip_menu.items.len() {
            error!("handle_cmsg_gossip_select_option: received a non-existing option_id (index {} but menu only has {} items)", cmsg.option_index, gossip_menu.items.len());
            return;
        }

        let gossip_menu_option = &gossip_menu_record.options[cmsg.option_index as usize];

        match gossip_menu_option.option_type {
            GossipMenuOptionType::Innkeeper => {
                session.close_gossip_menu();
                let packet = ServerMessage::new(SmsgBinderConfirm {
                    guid: cmsg.guid,
                });
                session.send(&packet).unwrap();
            },
            GossipMenuOptionType::Trainer => {
                let Some(trainer_type) = creature_template.trainer_type else {
                    error!("handle_cmsg_gossip_select_option: received a trainer option but creature is not a trainer");
                    session.close_gossip_menu();
                    return;
                };

                let Some(player_entity_id) = session.player_entity_id() else {
                    error!("handle_cmsg_gossip_select_option: no player_entity_id in session");
                    return;
                };

                let is_valid_trainer = map.world().run(|v_player: View<Player>| {
                    let Ok(player) = v_player.get(player_entity_id) else { return false; };

                    match trainer_type {
                        TrainerType::Class => {
                            if player.class() != creature_template.trainer_class.unwrap() {
                                let gossip_text_id = match creature_template.trainer_class.unwrap() {
                                    CharacterClass::None => 0,
                                    CharacterClass::Warrior => 4985,
                                    CharacterClass::Paladin => 1635,
                                    CharacterClass::Hunter => 10090,
                                    CharacterClass::Rogue => 4797,
                                    CharacterClass::Priest => 4436,
                                    CharacterClass::Shaman => 5003,
                                    CharacterClass::Mage => 328,
                                    CharacterClass::Warlock => 5836,
                                    CharacterClass::Druid => 4913,
                                };

                                OpcodeHandler::send_gossip_text(&cmsg.guid, gossip_text_id, session.clone());
                                return false;
                            }
                        },
                        _ => {
                            // TODO: same for mount with race, tradeskills and pets for non-hunter (see Mangos' IsTrainerOf second half)
                            warn!("add checks for trainer type {trainer_type:?} if needed");
                        },
                    }

                    true
                });

                if !is_valid_trainer {
                    return;
                }

                let Some(trainer_spells) = world_context.data_store.get_trainer_spells_by_creature_entry(creature_template.entry) else {
                    error!("handle_cmsg_gossip_select_option: received a trainer option but no spells found for entry {}", creature_template.entry);
                    session.close_gossip_menu();
                    return;
                };

                let spells_for_packet: Vec<TrainerSpell> = map.world().run(|v_player: View<Player>| {
                    let Ok(player) = v_player.get(session.player_entity_id().unwrap()) else {
                        return Vec::new();
                    };

                    trainer_spells.into_iter()
                        .filter_map(|spell| {
                            if !player.can_train_spell(spell.spell_id, world_context.clone()) {
                                return None;
                            }

                            let required_level = world_context.data_store.get_skill_required_level_for_player(spell.spell_id, player.race_bit(), player.class_bit()).unwrap_or(0);
                            let required_level = required_level.max(spell.required_level);

                            Some(TrainerSpell {
                                spell_id: spell.spell_id,
                                state: spell.state_for_player(player, required_level),
                                cost: spell.spell_cost,
                                can_learn_primary_profession_first_rank: false, // FIXME: professions
                                enable_learn_primary_profession_button: false, // FIXME: professions
                                required_level: required_level as u8,
                                required_skill: spell.required_skill,
                                required_skill_value: spell.required_skill_value,
                                previous_spell: 0, // FIXME: spell chains
                                required_required_spell: 0, // FIXME: spell chains
                                unk: 0, // always 0 in MaNGOS
                            })
                        })
                        .collect()
                });

                let packet = ServerMessage::new(SmsgTrainerList {
                    trainer_guid: cmsg.guid,
                    trainer_type: trainer_type as u32,
                    spell_count: spells_for_packet.len() as u32,
                    spells: spells_for_packet,
                    title: NullString::from("Hello! Ready for some training?"),
                });
                session.send(&packet).unwrap();
            }
            ot => warn!("handle_cmsg_gossip_select_option: received a non-implemented-yet option type {ot:?}"),
        };
    }

    pub fn handle_cmsg_trainer_buy_spell(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgTrainerBuySpell = ClientMessage::read_as(data).unwrap();

        let Some(map) = session.current_map() else {
            error!("handle_cmsg_trainer_buy_spell: session has no map");
            return;
        };

        let Some(target_entity_id) = map.lookup_entity_ecs(&cmsg.trainer_guid) else {
            error!(
                "handle_cmsg_trainer_buy_spell: map has no EntityId for cmsg.guid (guid: {:?})",
                cmsg.trainer_guid
            );
            return;
        };

        let Ok(creature_template) = map.world().run(|v_creature: View<Creature>| {
            v_creature.get(target_entity_id).map(|c| c.template.clone())
        }) else {
            warn!("handle_cmsg_trainer_buy_spell: target is not a creature");
            return;
        };

        if creature_template.trainer_type.is_none() {
            error!("handle_cmsg_trainer_buy_spell: creature is not a trainer");
            return;
        };

        let Some(trainer_spells) = world_context
            .data_store
            .get_trainer_spells_by_creature_entry(creature_template.entry)
        else {
            error!(
                "handle_cmsg_trainer_buy_spell: no spells found for entry {}",
                creature_template.entry
            );
            return;
        };

        let Some(trainer_spell) = trainer_spells
            .iter()
            .find(|tsp| tsp.spell_id == cmsg.spell_id)
        else {
            error!("handle_cmsg_trainer_buy_spell: request spell not trained by the trainer");
            return;
        };

        map.world().run(|mut vm_player: ViewMut<Player>| {
            let Ok(mut player) = (&mut vm_player).get(session.player_entity_id().unwrap()) else {
                error!("handle_cmsg_trainer_buy_spell: no player in session");
                return;
            };

            // FIXME: take reputation into account for a potential discount (see Player::GetReputationPriceDiscount(Creature*) in MaNGOS)

            if player.money() < trainer_spell.spell_cost {
                return;
            }

            player.modify_money(-1 * trainer_spell.spell_cost as i32);

            let packet = ServerMessage::new(SmsgPlaySpellVisual {
                caster_guid: cmsg.trainer_guid,
                spell_art_kit: 0xB3,
            }); // Visual effect on trainer
            session.send(&packet).unwrap();

            let packet = ServerMessage::new(SmsgPlaySpellImpact {
                caster_guid: player.guid(),
                spell_art_kit: 0x16A,
            }); // Visual effect on player
            session.send(&packet).unwrap();

            player.add_spell(trainer_spell.spell_id);

            let packet = ServerMessage::new(SmsgLearnedSpell {
                spell_id: trainer_spell.spell_id,
            });
            session.send(&packet).unwrap();

            let packet = ServerMessage::new(SmsgTrainerBuySucceeded {
                trainer_guid: cmsg.trainer_guid,
                spell_id: trainer_spell.spell_id,
            });
            session.send(&packet).unwrap();
        });
    }

    pub fn handle_cmsg_list_inventory(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgListInventory = ClientMessage::read_as(data).unwrap();

        let Some(map) = session.current_map() else {
            error!("handle_cmsg_list_inventory: session has no map");
            return;
        };

        let Some(target_entity_id) = map.lookup_entity_ecs(&cmsg.vendor_guid) else {
            error!(
                "handle_cmsg_list_inventory: map has no EntityId for cmsg.vendor_guid (guid: {:?})",
                cmsg.vendor_guid
            );
            return;
        };

        let Ok(creature_template) = map.world().run(|v_creature: View<Creature>| {
            v_creature.get(target_entity_id).map(|c| c.template.clone())
        }) else {
            warn!("handle_cmsg_list_inventory: target is not a creature");
            return;
        };

        let Some(inventory_items) = world_context
            .data_store
            .get_vendor_inventory_by_creature_entry(creature_template.entry)
        else {
            error!(
                "handle_cmsg_list_inventory: no vendor inventory found for entry {}",
                creature_template.entry
            );
            let packet = ServerMessage::new(SmsgListInventory::empty(cmsg.vendor_guid));
            session.send(&packet).unwrap();
            return;
        };

        let items_for_packet: Vec<InventoryItem> = inventory_items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let Some(item_template) = world_context.data_store.get_item_template(item.item_id)
                else {
                    // Note: this should never happen because we have a foreign key referencing item_templates in DB
                    warn!(
                        "found unknown item id {} on vendor (creature template entry {})",
                        item.item_id, creature_template.entry
                    );
                    return None;
                };

                Some(InventoryItem {
                    index: (index + 1) as u32,
                    item_id: item.item_id,
                    item_display_id: item_template.display_id,
                    item_count_at_vendor: item
                        .max_count
                        .filter(|&count| count > 0)
                        .unwrap_or(0xFFFFFFFF),
                    price: item_template.buy_price, // FIXME: take reputation into account for a potential discount (see Player::GetReputationPriceDiscount(Creature*) in MaNGOS)
                    max_durability: item_template.max_durability,
                    buy_count: item_template.buy_count,
                    extended_cost_id: item.extended_cost_id.unwrap_or(0),
                })
            })
            .collect();

        if items_for_packet.is_empty() {
            let packet = ServerMessage::new(SmsgListInventory::empty(cmsg.vendor_guid));
            session.send(&packet).unwrap();
        } else {
            let packet = ServerMessage::new(SmsgListInventory::from_items(
                cmsg.vendor_guid,
                items_for_packet,
            ));
            session.send(&packet).unwrap();
        }
    }

    pub fn handle_cmsg_buy_item(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgBuyItem = ClientMessage::read_as(data).unwrap();

        // TODO: handle extended cost
        // TODO: handle limited items (check current stock, periodically restock, ...)
        let Some(map) = session.current_map() else {
            error!("handle_cmsg_buy_item: session has no map");
            return;
        };

        let Some(target_entity_id) = map.lookup_entity_ecs(&cmsg.vendor_guid) else {
            error!(
                "handle_cmsg_buy_item: map has no EntityId for cmsg.vendor_guid (guid: {:?})",
                cmsg.vendor_guid
            );
            return;
        };

        let Ok(creature_template) = map.world().run(|v_creature: View<Creature>| {
            v_creature.get(target_entity_id).map(|c| c.template.clone())
        }) else {
            warn!("handle_cmsg_buy_item: target is not a creature");
            return;
        };

        let Some(inventory_items) = world_context
            .data_store
            .get_vendor_inventory_by_creature_entry(creature_template.entry)
        else {
            error!(
                "handle_cmsg_buy_item: no vendor inventory found for entry {}",
                creature_template.entry
            );
            let packet = ServerMessage::new(SmsgListInventory::empty(cmsg.vendor_guid));
            session.send(&packet).unwrap();
            return;
        };

        let Some(bought_index) = inventory_items
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                if item.item_id == cmsg.item_id {
                    Some(index)
                } else {
                    None
                }
            })
        else {
            warn!(
                "handle_cmsg_buy_item: item {} not found in vendor inventory",
                cmsg.item_id
            );
            return;
        };

        let Some(item_template) = world_context.data_store.get_item_template(cmsg.item_id) else {
            // Note: this should never happen because we have a foreign key referencing item_templates in DB
            warn!(
                "found unknown item id {} on vendor (creature template entry {})",
                cmsg.item_id, creature_template.entry
            );
            return;
        };

        // FIXME: take reputation into account for a potential discount (see Player::GetReputationPriceDiscount(Creature*) in MaNGOS)
        let price = item_template.buy_price * cmsg.count as u32;

        map.world().run(
            |mut vm_player: ViewMut<Player>, mut vm_attributes: ViewMut<Attributes>| {
                let Ok(mut player) = (&mut vm_player).get(session.player_entity_id().unwrap())
                else {
                    error!("handle_cmsg_buy_item: session has no player");
                    return;
                };

                let Ok(mut attributes) =
                    (&mut vm_attributes).get(session.player_entity_id().unwrap())
                else {
                    error!("handle_cmsg_buy_item: player has no AttributeModifiers component");
                    return;
                };

                if player.money() < price {
                    let packet = ServerMessage::new(SmsgBuyFailed {
                        vendor_guid: cmsg.vendor_guid,
                        item_id: cmsg.item_id,
                        param: None,
                        fail_reason: BuyFailedReason::NotEnoughtMoney,
                    });
                    session.send(&packet).unwrap();
                    return;
                }

                match player.auto_store_new_item(
                    cmsg.item_id,
                    cmsg.count.into(),
                    &mut attributes,
                ) {
                    Ok(inventory_slot) => {
                        player.modify_money(-1 * price as i32);
                        let packet = ServerMessage::new(SmsgBuyItem {
                            vendor_guid: cmsg.vendor_guid,
                            index: bought_index as u32,
                            new_count: 0xFFFFFFFF, // TODO: handle limited items
                            bought_count: cmsg.count as u32,
                        });

                        session.send(&packet).unwrap();

                        let total_count = player.inventory().get_item_count(cmsg.item_id);
                        let packet = ServerMessage::new(SmsgItemPushResult {
                            player_guid: player.guid(),
                            loot_source: 1, // 1 = from npc
                            is_created: 0,
                            is_visible_in_chat: 1,
                            bag_slot: 255, // FIXME: INVENTORY_SLOT_BAG_0
                            item_slot: inventory_slot,
                            item_id: cmsg.item_id,
                            item_suffix_factor: 0,      // FIXME
                            item_random_property_id: 0, // FIXME
                            count: cmsg.count as u32,
                            total_count_of_this_item_in_inventory: total_count,
                        });

                        session.send(&packet).unwrap();
                    }
                    Err(inventory_result) => {
                        let packet = ServerMessage::new(SmsgInventoryChangeFailure::build(
                            inventory_result,
                            None,
                            None,
                            None,
                        ));

                        session.send(&packet).unwrap();
                    }
                }
            },
        );
    }

    pub fn handle_cmsg_sell_item(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgSellItem = ClientMessage::read_as(data).unwrap();

        let Some(map) = session.current_map() else {
            error!("handle_cmsg_sell_item: session has no map");
            return;
        };

        if map.lookup_entity_ecs(&cmsg.vendor_guid).is_none() {
            error!(
                "handle_cmsg_sell_item: map has no EntityId for cmsg.vendor_guid (guid: {:?})",
                cmsg.vendor_guid
            );
            return;
        }

        map.world().run(|mut vm_player: ViewMut<Player>, mut vm_attributes: ViewMut<Attributes>| {
            let Ok(mut player) = (&mut vm_player).get(session.player_entity_id().unwrap()) else {
                error!("handle_cmsg_sell_item: session has no player");
                return;
            };

            let Ok(mut attributes) =
                (&mut vm_attributes).get(session.player_entity_id().unwrap())
            else {
                error!("handle_cmsg_sell_item: player has no AttributeModifiers component");
                return;
            };

            // FIXME: don't sell item if template.sell_price == 0

            let slot_to_remove: Option<u32>;
            let item_template: Option<&ItemTemplate>;
            {
                let inventory = player.inventory_mut();

                let Some((slot, item)) = inventory.get_mut_by_guid(cmsg.item_guid) else {
                    let packet = ServerMessage::new(SmsgSellItem {
                        vendor_guid: cmsg.vendor_guid,
                        item_guid: cmsg.item_guid,
                        param: None,
                        fail_reason: SellFailedReason::CantFindItem,
                    });

                    session.send(&packet).unwrap();
                    return;
                };

                if cmsg.count > 0 && cmsg.count as u32 != item.stack_count() {
                    warn!("handle_cmsg_sell_item: unhandled case where cmsg.count > 0 && cmsg.count != item.stack_count() - TODO!");
                    return;
                }

                slot_to_remove = Some(*slot);
                item_template = world_context.data_store.get_item_template(item.entry());
            }

            let Some(item_template) = item_template else {
                error!("handle_cmsg_sell_item: attempt to sell unknown item id (item guid: {:?})", cmsg.item_guid);
                return;
            };

            if item_template.sell_price == 0 {
                let packet = ServerMessage::new(SmsgSellItem {
                    vendor_guid: cmsg.vendor_guid,
                    item_guid: cmsg.item_guid,
                    param: None,
                    fail_reason: SellFailedReason::CantSellItem,
                });

                session.send(&packet).unwrap();
                return;
            }

            if let Some(slot_to_remove) = slot_to_remove {
                if let Some(sold_item) = player.remove_item(slot_to_remove, &mut attributes) {
                    let price = item_template.sell_price * sold_item.stack_count();
                    player.modify_money(price as i32);
                }

                // TODO: Implement buyback
            }
        });
    }
}
