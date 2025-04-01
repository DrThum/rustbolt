ALTER TABLE characters ADD COLUMN bindpoint_map_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE characters ADD COLUMN bindpoint_area_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE characters ADD COLUMN bindpoint_position_x REAL NOT NULL DEFAULT 0.0;
ALTER TABLE characters ADD COLUMN bindpoint_position_y REAL NOT NULL DEFAULT 0.0;
ALTER TABLE characters ADD COLUMN bindpoint_position_z REAL NOT NULL DEFAULT 0.0;
ALTER TABLE characters ADD COLUMN bindpoint_orientation REAL NOT NULL DEFAULT 0.0;
