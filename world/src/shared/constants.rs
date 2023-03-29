use enumn::N;

#[allow(dead_code)]
#[derive(Clone, Copy, N)]
pub enum InventoryType {
    NonEquip = 0,
    Head = 1,
    Neck = 2,
    Shoulders = 3,
    Body = 4,
    Chest = 5,
    Waist = 6,
    Legs = 7,
    Feet = 8,
    Wrists = 9,
    Hands = 10,
    Finger = 11,
    Trinket = 12,
    Weapon = 13,
    Shield = 14,
    Ranged = 15,
    Cloak = 16,
    TwoHandWeapon = 17,
    Bag = 18,
    Tabard = 19,
    Robe = 20,
    WeaponMainHand = 21,
    WeaponOffHand = 22,
    Holdable = 23,
    Ammo = 24,
    Thrown = 25,
    RangedRight = 26,
    Quiver = 27,
    Relic = 28,
}

#[allow(dead_code)]
#[derive(Clone, Copy, N)]
pub enum Gender {
    Male = 0,
    Female = 1,
}

#[allow(dead_code)]
#[derive(Clone, Copy, N)]
pub enum CharacterRace {
    None = 0,
    Human = 1,
    Orc = 2,
    Dwarf = 3,
    NightElf = 4,
    UndeadPlayer = 5,
    Tauren = 6,
    Gnome = 7,
    Troll = 8,
    //Goblin         = 9,
    BloodElf = 10,
    Draenei = 11,
    //FelOrc        = 12,
    //Naga           = 13,
    //Broken         = 14,
    //Skeleton       = 15,
    //ForestTroll   = 18,
}

#[allow(dead_code)]
#[derive(Clone, Copy, N)]
pub enum CharacterClass {
    None = 0,
    Warrior = 1,
    Paladin = 2,
    Hunter = 3,
    Rogue = 4,
    Priest = 5,
    Shaman = 7,
    Mage = 8,
    Warlock = 9,
    Druid = 11,
}

#[allow(dead_code)]
#[derive(Clone, Copy, N)]
pub enum PowerType {
    Mana = 0,
    Rage = 1,
    Focus = 2,
    Energy = 3,
}
