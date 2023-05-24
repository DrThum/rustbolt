CREATE TABLE creature_spawns(
  guid INTEGER PRIMARY KEY,
  entry INTEGER NOT NULL,
  map INTEGER NOT NULL,
  position_x REAL NOT NULL,
  position_y REAL NOT NULL,
  position_z REAL NOT NULL,
  orientation REAL NOT NULL,
  FOREIGN KEY(entry) REFERENCES creature_templates(entry)
);
