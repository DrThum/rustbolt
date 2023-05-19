use enumflags2::bitflags;
use enumn::N;

pub const STANDARD_ADDON_CRC: u32 = 0x1C776D01;

pub const ADDON_PUBLIC_KEY: [u8; 256] = [
    0xC3, 0x5B, 0x50, 0x84, 0xB9, 0x3E, 0x32, 0x42, 0x8C, 0xD0, 0xC7, 0x48, 0xFA, 0x0E, 0x5D, 0x54,
    0x5A, 0xA3, 0x0E, 0x14, 0xBA, 0x9E, 0x0D, 0xB9, 0x5D, 0x8B, 0xEE, 0xB6, 0x84, 0x93, 0x45, 0x75,
    0xFF, 0x31, 0xFE, 0x2F, 0x64, 0x3F, 0x3D, 0x6D, 0x07, 0xD9, 0x44, 0x9B, 0x40, 0x85, 0x59, 0x34,
    0x4E, 0x10, 0xE1, 0xE7, 0x43, 0x69, 0xEF, 0x7C, 0x16, 0xFC, 0xB4, 0xED, 0x1B, 0x95, 0x28, 0xA8,
    0x23, 0x76, 0x51, 0x31, 0x57, 0x30, 0x2B, 0x79, 0x08, 0x50, 0x10, 0x1C, 0x4A, 0x1A, 0x2C, 0xC8,
    0x8B, 0x8F, 0x05, 0x2D, 0x22, 0x3D, 0xDB, 0x5A, 0x24, 0x7A, 0x0F, 0x13, 0x50, 0x37, 0x8F, 0x5A,
    0xCC, 0x9E, 0x04, 0x44, 0x0E, 0x87, 0x01, 0xD4, 0xA3, 0x15, 0x94, 0x16, 0x34, 0xC6, 0xC2, 0xC3,
    0xFB, 0x49, 0xFE, 0xE1, 0xF9, 0xDA, 0x8C, 0x50, 0x3C, 0xBE, 0x2C, 0xBB, 0x57, 0xED, 0x46, 0xB9,
    0xAD, 0x8B, 0xC6, 0xDF, 0x0E, 0xD6, 0x0F, 0xBE, 0x80, 0xB3, 0x8B, 0x1E, 0x77, 0xCF, 0xAD, 0x22,
    0xCF, 0xB7, 0x4B, 0xCF, 0xFB, 0xF0, 0x6B, 0x11, 0x45, 0x2D, 0x7A, 0x81, 0x18, 0xF2, 0x92, 0x7E,
    0x98, 0x56, 0x5D, 0x5E, 0x69, 0x72, 0x0A, 0x0D, 0x03, 0x0A, 0x85, 0xA2, 0x85, 0x9C, 0xCB, 0xFB,
    0x56, 0x6E, 0x8F, 0x44, 0xBB, 0x8F, 0x02, 0x22, 0x68, 0x63, 0x97, 0xBC, 0x85, 0xBA, 0xA8, 0xF7,
    0xB5, 0x40, 0x68, 0x3C, 0x77, 0x86, 0x6F, 0x4B, 0xD7, 0x88, 0xCA, 0x8A, 0xD7, 0xCE, 0x36, 0xF0,
    0x45, 0x6E, 0xD5, 0x64, 0x79, 0x0F, 0x17, 0xFC, 0x64, 0xDD, 0x10, 0x6F, 0xF3, 0xF5, 0xE0, 0xA6,
    0xC3, 0xFB, 0x1B, 0x8C, 0x29, 0xEF, 0x8E, 0xE5, 0x34, 0xCB, 0xD1, 0x2A, 0xCE, 0x79, 0xC3, 0x9A,
    0x0D, 0x36, 0xEA, 0x01, 0xE0, 0xAA, 0x91, 0x20, 0x54, 0xF0, 0x72, 0xD8, 0x1E, 0xC7, 0x89, 0xD2,
];

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

#[allow(dead_code)]
#[derive(Clone, Copy, N, Hash, Eq, PartialEq)]
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
pub enum InventorySlot {
    EquipmentHead = 0,
    EquipmentNeck = 1,
    EquipmentShoulders = 2,
    EquipmentBody = 3,
    EquipmentChest = 4,
    EquipmentWaist = 5,
    EquipmentLegs = 6,
    EquipmentFeet = 7,
    EquipmentWrists = 8,
    EquipmentHands = 9,
    EquipmentFinger1 = 10,
    EquipmentFinger2 = 11,
    EquipmentTrinket1 = 12,
    EquipmentTrinket2 = 13,
    EquipmentBack = 14,
    EquipmentMainhand = 15,
    EquipmentOffhand = 16,
    EquipmentRanged = 17,
    EquipmentTabard = 18,
}

#[allow(dead_code)]
impl InventorySlot {
    // Gear
    pub const EQUIPMENT_START: u32 = 0;
    pub const EQUIPMENT_END: u32 = 19;

    // Bag slots (bottom right - 4 slots)
    pub const BAG_START: u32 = 19;
    pub const BAG_END: u32 = 23;

    // Backpack items (first bag - 16 slots)
    pub const BACKPACK_START: u32 = 23;
    pub const BACKPACK_END: u32 = 39;

    // Bank items (first bag - 28 slots)
    pub const BANK_START: u32 = 39;
    pub const BANK_END: u32 = 67;

    // Bank bag slots (7 slots)
    pub const BANK_BAG_START: u32 = 67;
    pub const BANK_BAG_END: u32 = 74;

    // Buy back slots at NPCs (12 slots)
    pub const BUY_BACK_START: u32 = 74;
    pub const BUY_BACK_END: u32 = 86;

    // Key ring (32 slots)
    pub const KEY_RING_START: u32 = 86;
    pub const KEY_RING_END: u32 = 118;
}

#[allow(dead_code)]
#[derive(Clone, Copy, N, PartialEq, Eq, Debug)]
pub enum HighGuidType {
    ItemOrContainer = 0x4000,
    Player = 0x0000,
    Gameobject = 0xF110,
    Transport = 0xF120, // for GAMEOBJECT_TYPE_TRANSPORT
    Unit = 0xF130,
    Pet = 0xF140,
    Dynamicobject = 0xF100,
    Corpse = 0xF101,
    MoTransport = 0x1FC0, // for GAMEOBJECT_TYPE_MO_TRANSPORT
    Group = 0x1F50,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum ObjectTypeId {
    Object = 0,
    Item = 1,
    Container = 2,
    Unit = 3,
    Player = 4,
    GameObject = 5,
    DynamicObject = 6,
    Corpse = 7,
}

#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, N)]
pub enum ObjectTypeMask {
    Object = 0x0001,
    Item = 0x0002,
    Container = 0x0004,
    Unit = 0x0008,
    Player = 0x0010,
    Gameobject = 0x0020,
    Dynamicobject = 0x0040,
    Corpse = 0x0080,
}

pub const MAX_ITEM_TEMPLATE_STATS: u32 = 10;
pub const MAX_ITEM_TEMPLATE_DAMAGES: u32 = 5;
pub const MAX_ITEM_TEMPLATE_SPELLS: u32 = 5;
pub const MAX_ITEM_TEMPLATE_SOCKETS: u32 = 3;

#[allow(dead_code)]
#[derive(N)]
pub enum ItemClass {
    Consumable = 0,
    Container = 1,
    Weapon = 2,
    Gem = 3,
    Armor = 4,
    Reagent = 5,
    Projectile = 6,
    TradeGoods = 7,
    Generic = 8,
    Recipe = 9,
    Money = 10,
    Quiver = 11,
    Quest = 12,
    Key = 13,
    Permanent = 14,
    Misc = 15,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassConsumable {
    Consumable = 0,
    Potion = 1,
    Elixir = 2,
    Flask = 3,
    Scroll = 4,
    Food = 5,
    ItemEnhancement = 6,
    Bandage = 7,
    ConsumableOther = 8,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassContainer {
    Container = 0,
    SoulContainer = 1,
    HerbContainer = 2,
    EnchantingContainer = 3,
    EngineeringContainer = 4,
    GemContainer = 5,
    MiningContainer = 6,
    LeatherworkingContainer = 7,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassWeapon {
    Axe = 0,
    Axe2 = 1,
    Bow = 2,
    Gun = 3,
    Mace = 4,
    Mace2 = 5,
    Polearm = 6,
    Sword = 7,
    Sword2 = 8,
    Obsolete = 9,
    Staff = 10,
    Exotic = 11,
    Exotic2 = 12,
    Fist = 13,
    Misc = 14,
    Dagger = 15,
    Thrown = 16,
    Spear = 17,
    Crossbow = 18,
    Wand = 19,
    FishingPole = 20,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassGem {
    Red = 0,
    Blue = 1,
    Yellow = 2,
    Purple = 3,
    Green = 4,
    Orange = 5,
    Meta = 6,
    Simple = 7,
    Prismatic = 8,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassArmor {
    Misc = 0,
    Cloth = 1,
    Leather = 2,
    Mail = 3,
    Plate = 4,
    Buckler = 5,
    Shield = 6,
    Libram = 7,
    Idol = 8,
    Totem = 9,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassReagent {
    Reagent = 0,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassProjectile {
    Wand = 0,
    Bolt = 1,
    Arrow = 2,
    Bullet = 3,
    Thrown = 4,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassTradeGoods {
    TradeGoods = 0,
    Parts = 1,
    Explosives = 2,
    Devices = 3,
    Jewelcrafting = 4,
    Cloth = 5,
    Leather = 6,
    MetalStone = 7,
    Meat = 8,
    Herb = 9,
    Elemental = 10,
    TradeGoodsOther = 11,
    Enchanting = 12,
    Material = 13,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassGeneric {
    Generic = 0,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassRecipe {
    Book = 0,
    LeatherworkingPattern = 1,
    TailoringPattern = 2,
    EngineeringSchematic = 3,
    Blacksmithing = 4,
    CookingRecipe = 5,
    AlchemyRecipe = 6,
    FirstAidManual = 7,
    EnchantingFormula = 8,
    FishingManual = 9,
    JewelcraftingRecipe = 10,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassMoney {
    Money = 0,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassQuiver {
    Quiver0 = 0,
    Quiver1 = 1,
    Quiver = 2,
    AmmoPouch = 3,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassQuest {
    Quest = 0,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassKey {
    Key = 0,
    Lockpick = 1,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassPermanent {
    Permanent = 0,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ItemSubclassJunk {
    Junk = 0,
    JunkReagent = 1,
    JunkPet = 2,
    JunkHoliday = 3,
    JunkOther = 4,
    JunkMount = 5,
}

#[allow(dead_code)]
#[derive(N)]
pub enum UnitStandState {
    Stand = 0,
    Sit = 1,
    SitChair = 2,
    Sleep = 3,
    SitLowChair = 4,
    SitMediumChair = 5,
    SitHighChair = 6,
    Dead = 7,
    Kneel = 8,
    Custom = 9, // Depends on model animation. Submerge, freeze, hide, hibernate, rest
}

#[allow(dead_code)]
#[derive(N)]
pub enum SheathState {
    Unarmed = 0,
    Melee = 1,
    Ranged = 2,
}

#[allow(dead_code)]
#[derive(Debug, N)]
pub enum MapType {
    Common = 0,
    Instance = 1,
    Raid = 2,
    Battleground = 3,
    Arena = 4,
}

#[allow(dead_code)]
#[derive(N)]
pub enum ChatMessageType {
    Addon = 0xFFFFFFFF,
    System = 0x00,
    Say = 0x01,
    Party = 0x02,
    Raid = 0x03,
    Guild = 0x04,
    Officer = 0x05,
    Yell = 0x06,
    Whisper = 0x07,
    WhisperInform = 0x08,
    Reply = 0x09,
    Emote = 0x0A,
    TextEmote = 0x0B,
    MonsterSay = 0x0C,
    MonsterParty = 0x0D,
    MonsterYell = 0x0E,
    MonsterWhisper = 0x0F,
    MonsterEmote = 0x10,
    Channel = 0x11,
    ChannelJoin = 0x12,
    ChannelLeave = 0x13,
    ChannelList = 0x14,
    ChannelNotice = 0x15,
    ChannelNoticeUser = 0x16,
    Afk = 0x17,
    Dnd = 0x18,
    Ignored = 0x19,
    Skill = 0x1A,
    Loot = 0x1B,
    Money = 0x1C,
    Opening = 0x1D,
    Tradeskills = 0x1E,
    PetInfo = 0x1F,
    CombatMiscInfo = 0x20,
    CombatXpGain = 0x21,
    CombatHonorGain = 0x22,
    CombatFactionChange = 0x23,
    BgSystemNeutral = 0x24,
    BgSystemAlliance = 0x25,
    BgSystemHorde = 0x26,
    RaidLeader = 0x27,
    RaidWarning = 0x28,
    RaidBossWhisper = 0x29,
    RaidBossEmote = 0x2A,
    Filtered = 0x2B,
    Battleground = 0x2C,
    BattlegroundLeader = 0x2D,
    Restricted = 0x2E,
}
