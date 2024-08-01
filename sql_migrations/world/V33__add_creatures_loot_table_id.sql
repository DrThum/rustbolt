ALTER TABLE creature_templates ADD COLUMN loot_table_id INTEGER;

ALTER TABLE creature_templates RENAME TO creature_templates_old;

CREATE TABLE `creature_templates` (
  "entry" mediumint(8) NOT NULL DEFAULT '0',
  "name" char(100) NOT NULL DEFAULT '',
  "sub_name" char(100) DEFAULT NULL,
  "icon_name" char(100) DEFAULT NULL,
  "min_level" tinyint(3) NOT NULL DEFAULT '1',
  "max_level" tinyint(3) NOT NULL DEFAULT '1',
  "model_id1" mediumint(8) NOT NULL DEFAULT '0',
  "model_id2" mediumint(8) NOT NULL DEFAULT '0',
  "model_id3" mediumint(8) NOT NULL DEFAULT '0',
  "model_id4" mediumint(8) NOT NULL DEFAULT '0',
  "scale" float NOT NULL DEFAULT '1',
  "family" tinyint(4) NOT NULL DEFAULT '0',
  "type_id" tinyint(3) NOT NULL DEFAULT '0',
  "racial_leader" tinyint(3) NOT NULL DEFAULT '0',
  "type_flags" int(10) NOT NULL DEFAULT '0',
  "speed_walk" float NOT NULL DEFAULT '1',
  "speed_run" float NOT NULL DEFAULT '1.14286',
  "rank" tinyint(3) NOT NULL DEFAULT '0',
  "pet_spell_data_id" mediumint(8) NOT NULL DEFAULT '0',
  faction_template_id smallint(5) NOT NULL DEFAULT '0',
  npc_flags INTEGER NOT NULL DEFAULT 0,
  unit_flags INTEGER NOT NULL DEFAULT 0,
  dynamic_flags INTEGER NOT NULL DEFAULT 0,
  gossip_menu_id INTEGER DEFAULT NULL,
  movement_type INTEGER NOT NULL DEFAULT 0,
  melee_base_attack_time_ms INTEGER NOT NULL DEFAULT 2000,
  ranged_base_attack_time_ms INTEGER NOT NULL DEFAULT 2000,
  unit_class INTEGER NOT NULL DEFAULT 0,
  expansion INTEGER DEFAULT NULL,
  health_multiplier REAL NOT NULL DEFAULT 1,
  power_multiplier REAL NOT NULL DEFAULT 1,
  damage_multiplier REAL NOT NULL DEFAULT 1,
  armor_multiplier REAL NOT NULL DEFAULT 1,
  experience_multiplier REAL NOT NULL DEFAULT 1,
  base_damage_variance REAL NOT NULL DEFAULT 1.0,
  min_money_loot INTEGER NOT NULL DEFAULT 0,
  max_money_loot INTEGER NOT NULL DEFAULT 0,
  loot_table_id INTEGER,
  PRIMARY KEY ("entry"),
  FOREIGN KEY (loot_table_id) REFERENCES creature_loot_tables(id)
);

INSERT INTO creature_templates SELECT * FROM creature_templates_old;

-- The migration process gets stuck on this one for some reason :/
-- DROP TABLE creature_templates_old;
