CREATE TABLE loot_items (
  entity_type TEXT NOT NULL, -- npc, object, item, ...
  entity_id INTEGER NOT NULL,
  item_id INTEGER NOT NULL,
  icon_url TEXT NOT NULL,
  name TEXT NOT NULL,
  loot_percent_chance REAL NOT NULL,
  PRIMARY KEY (entity_type, entity_id, item_id)
);
