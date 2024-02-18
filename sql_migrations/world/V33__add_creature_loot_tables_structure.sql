CREATE TABLE IF NOT EXISTS creature_loot_tables(
  id INTEGER NOT NULL PRIMARY KEY,
  description TEXT
);

CREATE TABLE IF NOT EXISTS creature_loot_groups(
  loot_table_id INTEGER NOT NULL,
  group_id INTEGER NOT NULL,
  chance REAL NOT NULL,
  num_rolls_min INTEGER NOT NULL DEFAULT 1,
  num_rolls_max INTEGER NOT NULL DEFAULT 1,
  condition_id INTEGER,
  PRIMARY KEY (loot_table_id, group_id),
  FOREIGN KEY (loot_table_id) REFERENCES creature_loot_tables(id) ON DELETE CASCADE,
  CHECK (chance > 0 AND chance <= 100),
  CHECK (num_rolls_min > 0 AND num_rolls_max >= num_rolls_min)
);

CREATE TABLE IF NOT EXISTS creature_loot_items(
  loot_table_id INTEGER NOT NULL,
  group_id INTEGER NOT NULL,
  item_id INTEGER NOT NULL,
  chance REAL,
  condition_id INTEGER,
  PRIMARY KEY (loot_table_id, group_id, item_id),
  FOREIGN KEY (loot_table_id, group_id) REFERENCES creature_loot_groups(loot_table_id, group_id) ON DELETE CASCADE,
  CHECK (chance > 0 AND chance <= 100)
);
