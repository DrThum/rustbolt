CREATE TABLE loot_items (
  entity_type TEXT NOT NULL, -- npc, object, item, ...
  entity_id INTEGER NOT NULL,
  item_id INTEGER NOT NULL,
  icon_url TEXT NOT NULL,
  name TEXT NOT NULL,
  loot_percent_chance REAL NOT NULL,
  min_count INTEGER, -- 1 if NULL
  max_count INTEGER, -- 1 if NULL
  PRIMARY KEY (entity_type, entity_id, item_id)
);
