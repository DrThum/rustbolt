use std::error::Error;

use headless_chrome::{Browser, LaunchOptionsBuilder};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use regex::Regex;
use table_extract::Table;

use crate::{
    repositories::wowhead_cache::WowheadCacheRepository, wowhead::models::WowheadLootItem,
};

use super::models::{WowheadEntityType, WowheadLootTable};

pub fn get_loot_table(
    conn: &PooledConnection<SqliteConnectionManager>,
    entity_type: WowheadEntityType,
    id: u32,
) -> Option<WowheadLootTable> {
    if let Some(from_cache) = WowheadCacheRepository::get(conn, entity_type, id) {
        return Some(from_cache);
    }

    match browse_wowhead(entity_type, id) {
        Ok(from_wowhead) => {
            WowheadCacheRepository::save(conn, &from_wowhead);
            Some(from_wowhead)
        }
        Err(e) => {
            println!("error fetching the loot table from wowhead: {e:?}");
            None
        }
    }
}

fn browse_wowhead(
    entity_type: WowheadEntityType,
    id: u32,
) -> Result<WowheadLootTable, Box<dyn Error>> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(true)
            .devtools(false)
            .build()
            .unwrap(),
    )?;
    let tab = browser.new_tab()?;

    tab.navigate_to(&format!(
        "https://www.wowhead.com/tbc/{}={}",
        entity_type, id
    ))?;

    let icon_index = 1;
    let name_index = 2;
    let loot_percent_index = 12;

    let icon_regex = Regex::new(r"background-image: url\(&quot;(?<url>.*)&quot;\)").unwrap();
    let item_count_regex =
        Regex::new(r"<span [^>]*>(?<min_count>\d+)-(?<max_count>\d+)</span>").unwrap();
    let item_id_and_name_regex = Regex::new(r"item=(?<item_id>\d+)[^>]*>(?<name>[^<]*)").unwrap();
    let loot_percent_chance_regex = Regex::new(r"(?<loot_chance>[\d.]+)").unwrap(); // Sometimes it's <span class="tip">50</span>

    // TODO: Handle pagination (see npc 11502)
    let table_elem = tab.wait_for_element("#tab-drops > .listview-scroller-horizontal > .listview-scroller-vertical > table.listview-mode-default")?;
    let table_html = table_elem.get_content()?;

    let mut items: Vec<WowheadLootItem> = Vec::new();
    if let Some(table) = Table::find_first(&table_html) {
        for row in &table {
            let slice = row.as_slice();

            let icon = slice[icon_index].clone();
            let icon_url = &icon_regex.captures(&icon).unwrap()["url"];

            let captures = item_count_regex.captures(&icon);
            let (min_count, max_count) = captures
                .map(|captures| {
                    let min_count = captures["min_count"].parse::<u32>().unwrap();
                    let max_count = captures["max_count"].parse::<u32>().unwrap();

                    (Some(min_count), Some(max_count))
                })
                .unwrap_or((None, None));

            let name = slice[name_index].clone();
            let item_id = &item_id_and_name_regex.captures(&name).unwrap()["item_id"]
                .parse::<u32>()
                .unwrap();
            let name = &item_id_and_name_regex.captures(&name).unwrap()["name"];

            let loot_percent_chance = slice[loot_percent_index].clone();
            let loot_percent_chance = &loot_percent_chance_regex
                .captures(&loot_percent_chance)
                .unwrap()["loot_chance"];
            let loot_percent_chance = loot_percent_chance.parse::<f32>().unwrap();
            let loot_percent_chance = (loot_percent_chance * 100.).round() / 100.;

            items.push(WowheadLootItem {
                id: *item_id,
                icon_url: icon_url.to_string(),
                name: name.to_string(),
                loot_percent_chance,
                min_count,
                max_count,
            });
        }
    }

    Ok(WowheadLootTable {
        entity_type,
        id,
        items,
    })
}
