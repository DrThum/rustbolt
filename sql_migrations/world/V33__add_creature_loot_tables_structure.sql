-- Allows to use the same loot table for multiple creature templates
CREATE TABLE IF NOT EXISTS creature_loot_tables(
  id INTEGER NOT NULL PRIMARY KEY,
  description TEXT
);

-- Join table allowing to use multiple loot groups in a single creature loot table
-- but also reuse the same loot group in multiple creature loot tables
CREATE TABLE IF NOT EXISTS creature_loot_table_groups(
  creature_loot_table_id INTEGER NOT NULL,
  loot_group_id INTEGER NOT NULL,
  description TEXT,
  PRIMARY KEY (creature_loot_table_id, loot_group_id),
  FOREIGN KEY (creature_loot_table_id) REFERENCES creature_loot_tables(id) ON DELETE CASCADE,
  FOREIGN KEY (loot_group_id) REFERENCES loot_groups(id) ON DELETE CASCADE
);

-- Loot groups are used for creatures, gameobject, containers, ...
-- They can be used in multiple loot tables via creature_loot_table_groups (and gameobject, containers, ...)
CREATE TABLE IF NOT EXISTS loot_groups(
  id INTEGER NOT NULL,
  chance REAL NOT NULL,
  -- If selected, this group will be drawn from n times, between min and max
  num_rolls_min INTEGER NOT NULL DEFAULT 1,
  num_rolls_max INTEGER NOT NULL DEFAULT 1,
  condition_id INTEGER,
  PRIMARY KEY (id),
  CHECK (chance > 0 AND chance <= 100),
  CHECK (num_rolls_min > 0 AND num_rolls_max >= num_rolls_min)
);

-- One loot item belongs to one loot group
CREATE TABLE IF NOT EXISTS loot_items(
  group_id INTEGER NOT NULL,
  item_id INTEGER NOT NULL,
  chance REAL,
  -- If selected, this item will be drop n times, between min and max
  count_min INTEGER NOT NULL DEFAULT 1,
  count_max INTEGER NOT NULL DEFAULT 1,
  condition_id INTEGER,
  PRIMARY KEY (group_id, item_id),
  FOREIGN KEY (group_id) REFERENCES loot_groups(id) ON DELETE CASCADE,
  CHECK (chance > 0 AND chance <= 100)
);
