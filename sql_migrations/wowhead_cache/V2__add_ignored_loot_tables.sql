CREATE TABLE ignored_loot_tables (
  entity_type TEXT NOT NULL, -- npc, object, item, ...
  entity_id INTEGER NOT NULL,
  ignore_reason TEXT NOT NULL,
  PRIMARY KEY (entity_type, entity_id)
);
