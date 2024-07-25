ALTER TABLE creature_loot_tables RENAME TO loot_tables;
ALTER TABLE creature_loot_table_groups RENAME TO loot_table_groups;

ALTER TABLE loot_table_groups RENAME COLUMN creature_loot_table_id TO loot_table_id;
