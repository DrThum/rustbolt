CREATE TABLE seasonal_events(
  id INTEGER PRIMARY KEY,
  is_active INTEGER NOT NULL,
  description TEXT NOT NULL
);

INSERT INTO seasonal_events(id, is_active, description) VALUES
(1, 0, 'Midsummer Fire Festival'),
(2, 0, 'Feast of Winter Veil'),
(3, 0, 'Darkmoon Faire (Terokkar Forest)'),
(4, 0, 'Darkmoon Faire (Elwynn Forest)'),
(5, 0, 'Darkmoon Faire (Mulgore)'),
(6, 0, 'New Year''s Eve'),
(7, 0, 'Lunar Festival'),
(8, 0, 'Love is in the Air'),
(9, 0, 'Feast of Winter Veil - Presents'),
(10, 0, 'Children''s Week'),
(11, 0, 'Harvest Festival'),
(12, 0, 'Hallow''s End'),
(13, 0, 'Elemental Invasions'),
(14, 0, 'Stranglethorn Fishing Extravaganza - Announce'),
(15, 0, 'Stranglethorn Fishing Extravaganza'),
(16, 0, 'Gurubashi Arena Booty Run'),
(17, 0, 'Scourge Invasion'),
(18, 0, 'Call to Arms: Alterac Valley!'),
(19, 0, 'Call to Arms: Warsong Gulch!'),
(20, 0, 'Call to Arms: Arathi Basin!'),
(21, 0, 'Call to Arms: Eye of the Storm!'),
(22, 0, 'AQ War Effort'),
(26, 0, 'Brewfest'),
(27, 0, 'Nights'),
(28, 0, 'Noblegarden'),
(29, 0, 'Edge of Madness, Gri''lek'),
(30, 0, 'Edge of Madness, Hazza''rah'),
(31, 0, 'Edge of Madness, Renataki'),
(32, 0, 'Edge of Madness, Wushoolay'),
(33, 0, 'Arena Tournament'),
(34, 0, 'L70ETC Concert - Terrokar Forest (Blizzcon Event)'),
(36, 0, 'Stranglethorn Fishing Extravaganza - Turn-in'),
(41, 0, 'Darkmoon Faire (Elwynn Forest) - Building Stage 1'),
(42, 0, 'Darkmoon Faire (Elwynn Forest) - Building Stage 2'),
(43, 0, 'Darkmoon Faire (Terokkar Forest) - Building Stage 1'),
(44, 0, 'Darkmoon Faire (Terokkar Forest) - Building Stage 2'),
(45, 0, 'Brew of the Month - January'),
(46, 0, 'Brew of the Month - February'),
(47, 0, 'Brew of the Month - March'),
(48, 0, 'Brew of the Month - April'),
(49, 0, 'Brew of the Month - May'),
(50, 0, 'Brew of the Month - June'),
(51, 0, 'Brew of the Month - July'),
(52, 0, 'Brew of the Month - August'),
(53, 0, 'Brew of the Month - September'),
(54, 0, 'Brew of the Month - October'),
(55, 0, 'Brew of the Month - November'),
(56, 0, 'Brew of the Month - December'),
(57, 0, 'World''s End Tavern - Perry Gatner Announce'),
(58, 0, 'World''s End Tavern - Perry Gatner Standup Comedy'),
(59, 0, 'World''s End Tavern - L70ETC Concert Announce'),
(60, 0, 'World''s End Tavern - L70ETC Concert'),
(61, 0, 'Stormwind City - Stockades Jail Break'),
(62, 0, 'Darkmoon Faire (Mulgore) - Building Stage 1'),
(63, 0, 'Darkmoon Faire (Mulgore) - Building Stage 2'),
(64, 0, 'Grim Guzzler - L70ETC Pre-Concert'),
(65, 0, 'Grim Guzzler - L70ETC Concert');
