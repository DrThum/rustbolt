DELETE FROM creature_spawns WHERE entry IN (SELECT entry FROM creature_templates WHERE name LIKE '%TAR Ped%');
DELETE FROM creature_spawns WHERE entry IN (26760, 26075, 26012, 26324, 26328, 26307, 26329, 26331, 26330, 26325, 26332, 26327, 26326, 26309, 26309, 26309, 26007); -- Arena Tournament Server-specific spawns
