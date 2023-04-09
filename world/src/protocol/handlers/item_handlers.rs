use std::sync::Arc;

use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

impl OpcodeHandler {
    pub(crate) async fn handle_cmsg_item_query_single(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
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
                    required_enchantment_skill: item.required_disenchant_skill as i32,
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

        session.send(packet).await.unwrap();
    }
}
