use std::time::Duration;

use enumflags2::bitflags;
use enumn::N;
use log::warn;
use rusqlite::types::{FromSql, FromSqlError};
use strum::EnumIter;

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
    None = 2,
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
#[bitflags]
#[repr(u32)]
#[derive(N, Clone, Copy, Debug)]
pub enum CharacterRaceBit {
    Human = 1,
    Orc = 1 << 1,
    Dwarf = 1 << 2,
    NightElf = 1 << 3,
    UndeadPlayer = 1 << 4,
    Tauren = 1 << 5,
    Gnome = 1 << 6,
    Troll = 1 << 7,
    BloodElf = 1 << 9,
    Draenei = 1 << 10,
}

impl From<CharacterRace> for CharacterRaceBit {
    fn from(value: CharacterRace) -> Self {
        match value {
            CharacterRace::None => panic!("CharacterRace::None has no corresponding mask"),
            CharacterRace::Human => Self::Human,
            CharacterRace::Orc => Self::Orc,
            CharacterRace::Dwarf => Self::Dwarf,
            CharacterRace::NightElf => Self::NightElf,
            CharacterRace::UndeadPlayer => Self::UndeadPlayer,
            CharacterRace::Tauren => Self::Tauren,
            CharacterRace::Gnome => Self::Gnome,
            CharacterRace::Troll => Self::Troll,
            CharacterRace::BloodElf => Self::BloodElf,
            CharacterRace::Draenei => Self::Draenei,
        }
    }
}

impl From<u8> for CharacterRaceBit {
    fn from(value: u8) -> Self {
        let race = CharacterRace::n(value).expect("no matching character race");
        CharacterRaceBit::from(race)
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, N, PartialEq)]
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
#[bitflags]
#[repr(u32)]
#[derive(N, Clone, Copy, Debug)]
pub enum CharacterClassBit {
    Warrior = 1,
    Paladin = 1 << 1,
    Hunter = 1 << 2,
    Rogue = 1 << 3,
    Priest = 1 << 4,
    Shaman = 1 << 6,
    Mage = 1 << 7,
    Warlock = 1 << 8,
    Druid = 1 << 10,
}

impl From<CharacterClass> for CharacterClassBit {
    fn from(value: CharacterClass) -> Self {
        match value {
            CharacterClass::None => panic!("CharacterClass::None has no corresponding mask"),
            CharacterClass::Warrior => CharacterClassBit::Warrior,
            CharacterClass::Paladin => CharacterClassBit::Paladin,
            CharacterClass::Hunter => CharacterClassBit::Hunter,
            CharacterClass::Rogue => CharacterClassBit::Rogue,
            CharacterClass::Priest => CharacterClassBit::Priest,
            CharacterClass::Shaman => CharacterClassBit::Shaman,
            CharacterClass::Mage => CharacterClassBit::Mage,
            CharacterClass::Warlock => CharacterClassBit::Warlock,
            CharacterClass::Druid => CharacterClassBit::Druid,
        }
    }
}

impl From<u8> for CharacterClassBit {
    fn from(value: u8) -> Self {
        let class = CharacterClass::n(value).expect("no matching character class");
        CharacterClassBit::from(class)
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, N, EnumIter, Debug, PartialEq)]
pub enum PowerType {
    Health = -2,
    Mana = 0,
    Rage = 1,
    Focus = 2,
    Energy = 3,
    PetHappiness = 4,
}

pub const MAX_BASE_POWER_RAGE: u32 = 1000;
pub const MAX_BASE_POWER_FOCUS: u32 = 100;
pub const MAX_BASE_POWER_ENERGY: u32 = 100;
pub const MAX_BASE_POWER_PET_HAPPINESS: u32 = 1000000;

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
#[derive(Clone, Copy, N, Debug)]
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
    EquipmentMainHand = 15,
    EquipmentOffHand = 16,
    EquipmentRanged = 17,
    EquipmentTabard = 18,
}

impl PartialEq for InventorySlot {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::EquipmentFinger1, Self::EquipmentFinger2) => true,
            (Self::EquipmentFinger2, Self::EquipmentFinger1) => true,
            (Self::EquipmentTrinket1, Self::EquipmentTrinket2) => true,
            (Self::EquipmentTrinket2, Self::EquipmentTrinket1) => true,
            _ => *self as u32 == *other as u32,
        }
    }
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

    // Backpack
    pub const INVENTORY_SLOT_BAG_0: u32 = 255;
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
#[derive(Copy, Clone, Debug)]
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

pub const MAX_ITEM_TEMPLATE_STATS: usize = 10;
pub const MAX_ITEM_TEMPLATE_DAMAGES: usize = 5;
pub const MAX_ITEM_TEMPLATE_SPELLS: usize = 5;
pub const MAX_ITEM_TEMPLATE_SOCKETS: usize = 3;

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
#[derive(N, Debug, PartialEq)]
pub enum InventoryResult {
    Ok = 0,
    CantEquipLevelI = 1,
    CantEquipSkill = 2,
    ItemDoesntGoToSlot = 3,
    BagFull = 4,
    NonemptyBagOverOtherBag = 5,
    CantTradeEquipBags = 6,
    OnlyAmmoCanGoHere = 7,
    NoRequiredProficiency = 8,
    NoEquipmentSlotAvailable = 9,
    YouCanNeverUseThatItem = 10,
    YouCanNeverUseThatItem2 = 11,
    NoEquipmentSlotAvailable2 = 12,
    CantEquipWithTwohanded = 13,
    CantDualWield = 14,
    ItemDoesntGoIntoBag = 15,
    ItemDoesntGoIntoBag2 = 16,
    CantCarryMoreOfThis = 17,
    NoEquipmentSlotAvailable3 = 18,
    ItemCantStack = 19,
    ItemCantBeEquipped = 20,
    ItemsCantBeSwapped = 21,
    SlotIsEmpty = 22,
    ItemNotFound = 23,
    CantDropSoulbound = 24,
    OutOfRange = 25,
    TriedToSplitMoreThanCount = 26,
    CouldntSplitItems = 27,
    MissingReagent = 28,
    NotEnoughMoney = 29,
    NotABag = 30,
    CanOnlyDoWithEmptyBags = 31,
    DontOwnThatItem = 32,
    CanEquipOnly1Quiver = 33,
    MustPurchaseThatBagSlot = 34,
    TooFarAwayFromBank = 35,
    ItemLocked = 36,
    YouAreStunned = 37,
    YouAreDead = 38,
    CantDoRightNow = 39,
    IntBagError = 40,
    CanEquipOnly1Bolt = 41,
    CanEquipOnly1Ammopouch = 42,
    StackableCantBeWrapped = 43,
    EquippedCantBeWrapped = 44,
    WrappedCantBeWrapped = 45,
    BoundCantBeWrapped = 46,
    UniqueCantBeWrapped = 47,
    BagsCantBeWrapped = 48,
    AlreadyLooted = 49,
    InventoryFull = 50,
    BankFull = 51,
    ItemIsCurrentlySoldOut = 52,
    BagFull3 = 53,
    ItemNotFound2 = 54,
    ItemCantStack2 = 55,
    BagFull4 = 56,
    ItemSoldOut = 57,
    ObjectIsBusy = 58,
    None = 59,
    NotInCombat = 60,
    NotWhileDisarmed = 61,
    BagFull6 = 62,
    CantEquipRank = 63,
    CantEquipReputation = 64,
    TooManySpecialBags = 65,
    LootCantLootThatNow = 66,
    ItemUniqueEquipable = 67,
    VendorMissingTurnins = 68,
    NotEnoughHonorPoints = 69,
    NotEnoughArenaPoints = 70,
    ItemMaxCountSocketed = 71,
    MailBoundItem = 72,
    NoSplitWhileProspecting = 73,
    BagFull7 = 74,
    ItemMaxCountEquippedSocketed = 75,
    ItemUniqueEquippableSocketed = 76,
    TooMuchGold = 77,
    NotDuringArenaMatch = 78,
    CannotTradeThat = 79,
    PersonalArenaRatingTooLow = 80,
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
#[derive(Debug, N, Copy, Clone)]
pub enum MapType {
    Common = 0,
    Instance = 1,
    Raid = 2,
    Battleground = 3,
    Arena = 4,
}

#[allow(dead_code, clippy::enum_clike_unportable_variant)]
#[derive(N, PartialEq, Debug, Clone, Copy)]
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

#[allow(dead_code)]
#[derive(N, PartialEq, Clone, Copy)]
pub enum Language {
    Universal = 0,
    Orcish = 1,
    Darnassian = 2,
    Taurahe = 3,
    Dwarvish = 6,
    Common = 7,
    Demonic = 8,
    Titan = 9,
    Thalassian = 10,
    Draconic = 11,
    Kalimag = 12,
    Gnomish = 13,
    Troll = 14,
    Gutterspeak = 33,
    Draenei = 35,
    Zombie = 36,
    GnomishBinary = 37,
    GoblinBinary = 38,
}

#[allow(dead_code)]
#[derive(N)]
pub enum Emote {
    OneshotNone = 0,
    OneshotTalk = 1,
    OneshotBow = 2,
    OneshotWave = 3,
    OneshotCheer = 4,
    OneshotExclamation = 5,
    OneshotQuestion = 6,
    OneshotEat = 7,
    StateDance = 10,
    OneshotLaugh = 11,
    StateSleep = 12,
    StateSit = 13,
    OneshotRude = 14,
    OneshotRoar = 15,
    OneshotKneel = 16,
    OneshotKiss = 17,
    OneshotCry = 18,
    OneshotChicken = 19,
    OneshotBeg = 20,
    OneshotApplaud = 21,
    OneshotShout = 22,
    OneshotFlex = 23,
    OneshotShy = 24,
    OneshotPoint = 25,
    StateStand = 26,
    StateReadyunarmed = 27,
    StateWorkSheathed = 28,
    StatePoint = 29,
    StateNone = 30,
    OneshotWound = 33,
    OneshotWoundcritical = 34,
    OneshotAttackunarmed = 35,
    OneshotAttack1H = 36,
    OneshotAttack2Htight = 37,
    OneshotAttack2Hloose = 38,
    OneshotParryunarmed = 39,
    OneshotParryshield = 43,
    OneshotReadyunarmed = 44,
    OneshotReady1H = 45,
    OneshotReadybow = 48,
    OneshotSpellprecast = 50,
    OneshotSpellcast = 51,
    OneshotBattleroar = 53,
    OneshotSpecialattack1H = 54,
    OneshotKick = 60,
    OneshotAttackthrown = 61,
    StateStun = 64,
    StateDead = 65,
    OneshotSalute = 66,
    StateKneel = 68,
    StateUsestanding = 69,
    OneshotWaveNosheathe = 70,
    OneshotCheerNosheathe = 71,
    OneshotEatNosheathe = 92,
    StateStunNosheathe = 93,
    OneshotDance = 94,
    OneshotSaluteNosheath = 113,
    StateUsestandingNosheathe = 133,
    OneshotLaughNosheathe = 153,
    StateWork = 173,
    StateSpellprecast = 193,
    OneshotReadyrifle = 213,
    StateReadyrifle = 214,
    StateWorkMining = 233,
    StateWorkChopwood = 234,
    StateApplaud = 253,
    OneshotLiftoff = 254,
    OneshotYes = 273,
    OneshotNo = 274,
    OneshotTrain = 275,
    OneshotLand = 293,
    StateAtEase = 313,
    StateReady1H = 333,
    StateSpellkneelstart = 353,
    StateSubmerged = 373,
    OneshotSubmerge = 374,
    StateReady2H = 375,
    StateReadybow = 376,
    OneshotMountspecial = 377,
    StateTalk = 378,
    StateFishing = 379,
    OneshotFishing = 380,
    OneshotLoot = 381,
    StateWhirlwind = 382,
    StateDrowned = 383,
    StateHoldBow = 384,
    StateHoldRifle = 385,
    StateHoldThrown = 386,
    OneshotDrown = 387,
    OneshotStomp = 388,
    OneshotAttackoff = 389,
    OneshotAttackoffpierce = 390,
    StateRoar = 391,
    StateLaugh = 392,
    OneshotCreatureSpecial = 393,
    OneshotJumplandrun = 394,
    OneshotJumpend = 395,
    OneshotTalkNosheathe = 396,
    OneshotPointNosheathe = 397,
    StateCannibalize = 398,
    OneshotJumpstart = 399,
    StateDancespecial = 400,
    OneshotDancespecial = 401,
    OneshotCustomspell01 = 402,
    OneshotCustomspell02 = 403,
    OneshotCustomspell03 = 404,
    OneshotCustomspell04 = 405,
    OneshotCustomspell05 = 406,
    OneshotCustomspell06 = 407,
    OneshotCustomspell07 = 408,
    OneshotCustomspell08 = 409,
    OneshotCustomspell09 = 410,
    OneshotCustomspell10 = 411,
    StateExclaim = 412,
    StateSitChairMed = 415,
    StateSpelleffectHold = 422,
    StateEatNoSheathe = 423,
}

pub const MAX_CREATURE_TEMPLATE_MODELID: usize = 4;

pub const MAX_SPELL_TOTEMS: usize = 2;
pub const MAX_SPELL_REAGENTS: usize = 8;
pub const MAX_SPELL_EFFECTS: usize = 3;

#[allow(dead_code)]
#[derive(N, PartialEq, Debug, Eq, Hash)]
pub enum SpellEffect {
    None = 0,
    Instakill = 1,
    SchoolDamage = 2,
    Dummy = 3,
    PortalTeleport = 4,
    TeleportUnits = 5,
    ApplyAura = 6,
    EnvironmentalDamage = 7,
    PowerDrain = 8,
    HealthLeech = 9,
    Heal = 10,
    Bind = 11,
    Portal = 12,
    RitualBase = 13,
    RitualSpecialize = 14,
    RitualActivatePortal = 15,
    QuestComplete = 16,
    WeaponDamageNoSchool = 17,
    Resurrect = 18,
    AddExtraAttacks = 19,
    Dodge = 20,
    Evade = 21,
    Parry = 22,
    Block = 23,
    CreateItem = 24,
    Weapon = 25,
    Defense = 26,
    PersistentAreaAura = 27,
    Summon = 28,
    Leap = 29,
    Energize = 30,
    WeaponPercentDamage = 31,
    TriggerMissile = 32,
    OpenLock = 33,
    SummonChangeItem = 34,
    ApplyAreaAuraParty = 35,
    LearnSpell = 36,
    SpellDefense = 37,
    Dispel = 38,
    Language = 39,
    DualWield = 40,
    Jump = 41,
    JumpDest = 42,
    TeleportUnitsFaceCaster = 43,
    SkillStep = 44,
    AddHonor = 45,
    Spawn = 46,
    TradeSkill = 47,
    Stealth = 48,
    Detect = 49,
    TransDoor = 50,
    ForceCriticalHit = 51,
    GuaranteeHit = 52,
    EnchantItem = 53,
    EnchantItemTemporary = 54,
    TameCreature = 55,
    SummonPet = 56,
    LearnPetSpell = 57,
    WeaponDamage = 58,
    OpenLockItem = 59,
    Proficiency = 60,
    SendEvent = 61,
    PowerBurn = 62,
    Threat = 63,
    TriggerSpell = 64,
    HealthFunnel = 65,
    PowerFunnel = 66,
    HealMaxHealth = 67,
    InterruptCast = 68,
    Distract = 69,
    Pull = 70,
    Pickpocket = 71,
    AddFarsight = 72,
    Unused73 = 73,
    Unused74 = 74,
    HealMechanical = 75,
    SummonObjectWild = 76,
    ScriptEffect = 77,
    Attack = 78,
    Sanctuary = 79,
    AddComboPoints = 80,
    CreateHouse = 81,
    BindSight = 82,
    Duel = 83,
    Stuck = 84,
    SummonPlayer = 85,
    ActivateObject = 86,
    Unused87 = 87,
    Unused88 = 88,
    Unused89 = 89,
    Unused90 = 90,
    ThreatAll = 91,
    EnchantHeldItem = 92,
    Unused93 = 93,
    SelfResurrect = 94,
    Skinning = 95,
    Charge = 96,
    Unused97 = 97,
    KnockBack = 98,
    Disenchant = 99,
    Inebriate = 100,
    FeedPet = 101,
    DismissPet = 102,
    Reputation = 103,
    SummonObjectSlot1 = 104,
    SummonObjectSlot2 = 105,
    SummonObjectSlot3 = 106,
    SummonObjectSlot4 = 107,
    DispelMechanic = 108,
    SummonDeadPet = 109,
    DestroyAllTotems = 110,
    DurabilityDamage = 111,
    Unused112 = 112,
    ResurrectNew = 113,
    AttackMe = 114,
    DurabilityDamagePct = 115,
    SkinPlayerCorpse = 116,
    SpiritHeal = 117,
    Skill = 118,
    ApplyAreaAuraPet = 119,
    TeleportGraveyard = 120,
    NormalizedWeaponDmg = 121,
    Unused122 = 122,
    SendTaxi = 123,
    PlayerPull = 124,
    ModifyThreatPercent = 125,
    StealBeneficialBuff = 126,
    Prospecting = 127,
    ApplyAreaAuraFriend = 128,
    ApplyAreaAuraEnemy = 129,
    RedirectThreat = 130,
    PlaySound = 131,
    PlayMusic = 132,
    UnlearnSpecialization = 133,
    KillCreditGroup = 134,
    CallPet = 135,
    HealPct = 136,
    EnergizePct = 137,
    LeapBack = 138,
    ClearQuest = 139,
    ForceCast = 140,
    ForceCastWithValue = 141,
    TriggerSpellWithValue = 142,
    ApplyAreaAuraOwner = 143,
    KnockbackFromPosition = 144,
    PlayerPullTowardsDest = 145,
    Unused146 = 146,
    QuestFail = 147,
    Unused148 = 148,
    ChargeDest = 149,
    Unused150 = 150,
    TriggerSpell2 = 151,
    SummonReferAFriend = 152,
    CreateTamedPet = 153,
}

impl SpellEffect {
    pub fn is_positive(&self) -> bool {
        match self {
            Self::SchoolDamage => false,
            Self::TeleportUnits => true,
            Self::Heal => true,
            Self::Bind => true,
            Self::CreateItem => true,
            Self::OpenLock | Self::OpenLockItem => true,
            _ => {
                warn!("implement whether spell effect {self:?} is positive or negative");
                true
            }
        }
    }

    pub fn is_negative(&self) -> bool {
        !self.is_positive()
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, N)]
pub enum SpellTargetType {
    None = 0,
    TargetSelf = 1,
    RandomEnemyChainInArea = 2, // only one spell has that, but regardless, it's a target type after all
    RandomFriendChainInArea = 3,
    RandomUnitChainInArea = 4, // some plague spells that are infectious - maybe targets not-infected friends inrange
    Pet = 5,
    ChainDamage = 6,
    AreaeffectInstant = 7, // targets around provided destination point
    AreaeffectCustom = 8,
    InnkeeperCoordinates = 9, // uses in teleport to innkeeper spells
    Unk11 = 11,               // used by spell 4 'Word of Recall Other'
    AllEnemyInArea = 15,
    AllEnemyInAreaInstant = 16,
    TableXYZCoordinates = 17, // uses in teleport spells and some other
    EffectSelect = 18,        // highly depends on the spell effect
    AllPartyAroundCaster = 20,
    SingleFriend = 21,
    CasterCoordinates = 22, // used only in TargetA, target selection dependent from TargetB
    Gameobject = 23,
    InFrontOfCaster = 24,
    Duelvsplayer = 25,
    GameobjectItem = 26,
    Master = 27,
    AllEnemyInAreaChanneled = 28,
    Unk29 = 29,
    AllFriendlyUnitsAroundCaster = 30, // select friendly for caster object faction (in different original caster faction) in TargetB used only with TARGET_ALL_AROUND_CASTER and in self casting range in TargetA
    AllFriendlyUnitsInArea = 31,
    Minion = 32,
    AllParty = 33,
    AllPartyAroundCaster2 = 34, // used in Tranquility
    SingleParty = 35,
    AllHostileUnitsAroundCaster = 36,
    AreaeffectParty = 37,
    Script = 38,
    SelfFishing = 39,
    FocusOrScriptedGameobject = 40,
    TotemEarth = 41,
    TotemWater = 42,
    TotemAir = 43,
    TotemFire = 44,
    ChainHeal = 45,
    ScriptCoordinates = 46,
    DynamicObjectFront = 47,
    DynamicObjectBehind = 48,
    DynamicObjectLeftSide = 49,
    DynamicObjectRightSide = 50,
    AreaeffectGoAroundSource = 51,
    AreaeffectGoAroundDest = 52, // gameobject around destination, select by spell_script_target
    CurrentEnemyCoordinates = 53, // set unit coordinates as dest, only 16 target B imlemented
    LargeFrontalCone = 54,
    Unk55 = 55,
    AllRaidAroundCaster = 56,
    SingleFriend2 = 57,
    Unk58 = 58,
    Unk59 = 59,
    NarrowFrontalCone = 60,
    AreaeffectPartyAndClass = 61,
    Unk62 = 62,
    DuelvsplayerCoordinates = 63,
    InfrontOfVictim = 64,
    BehindVictim = 65, // used in teleport behind spells, caster/target dependent from spell effect
    RightFromVictim = 66,
    LeftFromVictim = 67,
    Unk70 = 70,
    RandomNearbyLoc = 72, // used in teleport onto nearby locations
    RandomCircumferencePoint = 73,
    Unk74 = 74,
    Unk75 = 75,
    DynamicObjectCoordinates = 76,
    SingleEnemy = 77,
    PointAtNorth = 78, // 78-85 possible _COORDINATES at radius with pi/4 step around target in unknown order, N?
    PointAtSouth = 79, // S?
    PointAtEast = 80, // 80/81 must be symmetric from line caster->target, E (base at 82/83, 84/85 order) ?
    PointAtWest = 81, // 80/81 must be symmetric from line caster->target, W (base at 82/83, 84/85 order) ?
    PointAtNe = 82,   // from spell desc: "(NE)"
    PointAtNw = 83,   // from spell desc: "(NW)"
    PointAtSe = 84,   // from spell desc: "(SE)"
    PointAtSw = 85,   // from spell desc: "(SW)"
    RandomNearbyDest = 86, // "Test Nearby Dest Random" - random around selected destination
    Self2 = 87,
    Unk88 = 88, // Smoke Flare(s) and Hurricane
    NoncombatPet = 90,
    Unk93 = 93,
}

#[allow(dead_code)]
#[derive(N, Copy, Clone, PartialEq, Debug)]
pub enum SkillType {
    None = 0,
    Frost = 6,
    Fire = 8,
    Arms = 26,
    Combat = 38,
    Subtlety = 39,
    Poisons = 40,
    Swords = 43,
    Axes = 44,
    Bows = 45,
    Guns = 46,
    BeastMastery = 50,
    Survival = 51,
    Maces = 54,
    TwoHandSwords = 55,
    Holy = 56,
    Shadow = 78,
    Defense = 95,
    LangCommon = 98,
    RacialDwarven = 101,
    LangOrcish = 109,
    LangDwarven = 111,
    LangDarnassian = 113,
    LangTaurahe = 115,
    DualWield = 118,
    RacialTauren = 124,
    OrcRacial = 125,
    RacialNightElf = 126,
    FirstAid = 129,
    FeralCombat = 134,
    Staves = 136,
    LangThalassian = 137,
    LangDraconic = 138,
    LangDemonTongue = 139,
    LangTitan = 140,
    LangOldTongue = 141,
    Survival2 = 142,
    RidingHorse = 148,
    RidingWolf = 149,
    RidingTiger = 150,
    RidingRam = 152,
    Swiming = 155,
    TwoHandMaces = 160,
    Unarmed = 162,
    Marksmanship = 163,
    Blacksmithing = 164,
    Leatherworking = 165,
    Alchemy = 171,
    TwoHandAxes = 172,
    Daggers = 173,
    Thrown = 176,
    Herbalism = 182,
    GenericDnd = 183,
    Retribution = 184,
    Cooking = 185,
    Mining = 186,
    PetImp = 188,
    PetFelhunter = 189,
    Tailoring = 197,
    Engineering = 202,
    PetSpider = 203,
    PetVoidwalker = 204,
    PetSuccubus = 205,
    PetInfernal = 206,
    PetDoomguard = 207,
    PetWolf = 208,
    PetCat = 209,
    PetBear = 210,
    PetBoar = 211,
    PetCrocilisk = 212,
    PetCarrionBird = 213,
    PetCrab = 214,
    PetGorilla = 215,
    PetRaptor = 217,
    PetTallstrider = 218,
    RacialUnded = 220,
    WeaponTalents = 222,
    Crossbows = 226,
    Spears = 227,
    Wands = 228,
    Polearms = 229,
    PetScorpid = 236,
    Arcane = 237,
    OpenLock = 242,
    PetTurtle = 251,
    Assassination = 253,
    Fury = 256,
    Protection = 257,
    BeastTraining = 261,
    Protection2 = 267,
    PetTalents = 270,
    PlateMail = 293,
    LangGnomish = 313,
    LangTroll = 315,
    Enchanting = 333,
    Demonology = 354,
    Affliction = 355,
    Fishing = 356,
    Enhancement = 373,
    Restoration = 374,
    ElementalCombat = 375,
    Skinning = 393,
    Mail = 413,
    Leather = 414,
    Cloth = 415,
    Shield = 433,
    FistWeapons = 473,
    RidingRaptor = 533,
    RidingMechanostrider = 553,
    RidingUndeadHorse = 554,
    Restoration2 = 573,
    Balance = 574,
    Destruction = 593,
    Holy2 = 594,
    Discipline = 613,
    Lockpicking = 633,
    PetBat = 653,
    PetHyena = 654,
    PetOwl = 655,
    PetWindSerpent = 656,
    LangGutterspeak = 673,
    RidingKodo = 713,
    RacialTroll = 733,
    RacialGnome = 753,
    RacialHuman = 754,
    Jewelcrafting = 755,
    RacialBloodelf = 756,
    PetEventRc = 758,
    LangDraenei = 759,
    RacialDraenei = 760,
    PetFelguard = 761,
    Riding = 762,
    PetDragonhawk = 763,
    PetNetherRay = 764,
    PetSporebat = 765,
    PetWarpStalker = 766,
    PetRavager = 767,
    PetSerpent = 768,
    Internal = 769,
}

#[allow(dead_code)]
#[derive(N, Clone, PartialEq, Debug)]
pub enum AbilityLearnType {
    None = 0,
    LearnedOnGetProfessionSkill = 1,
    LearnedOnGetRaceOrClassSkill = 2,
}

#[allow(dead_code)]
pub enum SkillRangeType {
    Language, // 300..300
    Level,    // 1..max skill for level
    Mono,     // 1..1, grey monolite bar
    Rank,     // 1..skill for known rank
    None,     // 0..0 always
}

#[derive(N)]
pub enum SkillCategory {
    None = -1,
    Attributes = 5,
    Weapon = 6,
    Class = 7,
    Armor = 8,
    SecondaryProfession = 9,
    Languages = 10,
    PrimaryProfession = 11,
    Generic = 12,
}

pub enum AbilitySkillFlags {
    NonTrainable = 0x100,
}

#[allow(dead_code)]
#[derive(N, Clone, Copy)]
pub enum ActionButtonType {
    Spell = 0x00,
    C = 0x01, // Click?
    Macro = 0x40,
    Cmacro = Self::C as isize | Self::Macro as isize,
    Item = 0x80,
}

impl FromSql for ActionButtonType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = value.as_i64()?;
        ActionButtonType::n(value).ok_or(FromSqlError::Other("invalid action_type".into()))
    }
}

pub const PLAYER_MAX_ACTION_BUTTONS: usize = 132;
pub const PLAYER_DEFAULT_COMBAT_REACH: f32 = 1.5;
pub const PLAYER_DEFAULT_BOUNDING_RADIUS: f32 = 1.5;
pub const BASE_MELEE_RANGE_OFFSET: f32 = 1.33;
pub const ATTACK_DISPLAY_DELAY: Duration = Duration::from_millis(200);

pub const FACTION_NUMBER_BASE_REPUTATION_MASKS: usize = 4;
pub const MAX_VISIBLE_REPUTATIONS: usize = 128;

#[allow(dead_code)]
#[repr(u32)]
pub enum UnitFlags {
    None = 0x00000000,
    ServerControlled = 0x00000001,
    NonAttackable = 0x00000002,
    RemoveClientControl = 0x00000004,
    PlayerControlled = 0x00000008, // Was PVP_ATTACKABLE
    Rename = 0x00000010,
    Preparation = 0x00000020,
    Unk6 = 0x00000040,
    NotAttackable1 = 0x00000080,
    OocNotAttackable = 0x00000100,
    Passive = 0x00000200,
    Looting = 0x00000400,
    PetInCombat = 0x00000800,
    Pvp = 0x00001000,
    Silenced = 0x00002000,
    CantSwim = 0x00004000,
    CanSwim = 0x00008000,
    NonAttackable2 = 0x00010000,
    Pacified = 0x00020000,
    Stunned = 0x00040000,
    InCombat = 0x00080000,
    OnTaxi = 0x00100000,
    Disarmed = 0x00200000,
    Confused = 0x00400000,
    Fleeing = 0x00800000,
    Possessed = 0x01000000,
    NotSelectable = 0x02000000,
    Skinnable = 0x04000000,
    Mount = 0x08000000,
    Unk28 = 0x10000000,
    PreventEmotesFromChatText = 0x20000000,
    Sheathe = 0x40000000,
    Immune = 0x80000000,
}

#[allow(dead_code)]
#[repr(u32)]
pub enum UnitFlags2 {
    FeignDeath = 0x00000001,
    HideBody = 0x00000002,
    IgnoreReputation = 0x00000004,
    ComprehendLang = 0x00000008,
    MirrorImage = 0x00000010,
    DontFadeIn = 0x00000020, // Model appears instantly
    ForceMove = 0x00000040,
}

#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum NpcFlags {
    Gossip = 0x00000001,
    QuestGiver = 0x00000002,
    Unk1 = 0x00000004,
    Unk2 = 0x00000008,
    Trainer = 0x00000010,
    TrainerClass = 0x00000020,
    TrainerProfession = 0x00000040,
    Vendor = 0x00000080,
    VendorAmmo = 0x00000100,
    VendorFood = 0x00000200,
    VendorPoison = 0x00000400,
    VendorReagent = 0x00000800,
    Repair = 0x00001000,
    FlightMaster = 0x00002000,
    SpiritHealer = 0x00004000,
    SpiritGuide = 0x00008000,
    Innkeeper = 0x00010000,
    Banker = 0x00020000,
    Petitioner = 0x00040000,
    TabardDesigner = 0x00080000,
    BattleMaster = 0x00100000,
    Auctioneer = 0x00200000,
    StableMaster = 0x00400000,
    GuildBanker = 0x00800000,
    SpellClick = 0x01000000,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, N)]
pub enum GossipMenuItemIcon {
    Chat = 0,      // white chat bubble
    Vendor = 1,    // brown bag
    Taxi = 2,      // flight
    Trainer = 3,   // book
    Interact1 = 4, // interaction wheel
    Interact2 = 5, // interaction wheel
    MoneyBag = 6,  // brown bag with yellow dot
    Talk = 7,      // white chat bubble with black dots
    Tabard = 8,    // tabard
    Battle = 9,    // two swords
    Dot = 10,      // yellow dot
    Chat11 = 11,   // This and below are most the same visual as GOSSIP_ICON_CHAT
    Chat12 = 12,   // but are still used for unknown reasons.
    Dot13 = 13,
    Dot14 = 14, // probably invalid
    Dot15 = 15, // probably invalid
    Dot16 = 16,
    Dot17 = 17,
    Dot18 = 18,
    Dot19 = 19,
    Dot20 = 20,
}

impl FromSql for GossipMenuItemIcon {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = value.as_i64()?;
        Self::n(value).ok_or(FromSqlError::Other(
            "invalid gossip menu item icon in gossip_menu_options".into(),
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, N)]
pub enum GossipMenuOptionType {
    None = 0,              // UNIT_NPC_FLAG_NONE                (0)
    Gossip = 1,            // UNIT_NPC_FLAG_GOSSIP              (1)
    QuestGiver = 2,        // UNIT_NPC_FLAG_QUESTGIVER          (2)
    Vendor = 3,            // UNIT_NPC_FLAG_VENDOR              (128)
    TaxiVendor = 4,        // UNIT_NPC_FLAG_TAXIVENDOR          (8192)
    Trainer = 5,           // UNIT_NPC_FLAG_TRAINER             (16)
    SpiritHealer = 6,      // UNIT_NPC_FLAG_SPIRITHEALER        (16384)
    SpiritGuide = 7,       // UNIT_NPC_FLAG_SPIRITGUIDE         (32768)
    Innkeeper = 8,         // UNIT_NPC_FLAG_INNKEEPER           (65536)
    Banker = 9,            // UNIT_NPC_FLAG_BANKER              (131072)
    Petitioner = 10,       // UNIT_NPC_FLAG_PETITIONER          (262144)
    TabardDesigner = 11,   // UNIT_NPC_FLAG_TABARDDESIGNER      (524288)
    Battlefield = 12,      // UNIT_NPC_FLAG_BATTLEFIELDPERSON   (1048576)
    Auctioneer = 13,       // UNIT_NPC_FLAG_AUCTIONEER          (2097152)
    Stablepet = 14,        // UNIT_NPC_FLAG_STABLE              (4194304)
    Armorer = 15,          // UNIT_NPC_FLAG_ARMORER             (4096)
    UnlearnTalents = 16, // UNIT_NPC_FLAG_TRAINER             (16) (bonus option for GOSSIP_OPTION_TRAINER)
    UnlearnPetSkills = 17, // UNIT_NPC_FLAG_TRAINER             (16) (bonus option for GOSSIP_OPTION_TRAINER)
}

impl FromSql for GossipMenuOptionType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = value.as_i64()?;
        Self::n(value).ok_or(FromSqlError::Other(
            "invalid gossip menu item option type in gossip_menu_options".into(),
        ))
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum WeaponAttackType {
    MainHand = 0,
    OffHand = 1,
    Ranged = 2,
}

pub const NUMBER_WEAPON_ATTACK_TYPES: usize = 3;

#[allow(dead_code)]
pub enum LifeCycleStage {
    Alive,
    Dead,
}

#[allow(dead_code)]
pub enum UnitDynamicFlag {
    None = 0x0000,
    Lootable = 0x0001,
    TrackUnit = 0x0002,
    Tapped = 0x0004, // Indicates the target as grey for the client
    Rooted = 0x0008,
    Specialinfo = 0x0010,
    Dead = 0x0020,
}

#[allow(dead_code)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MeleeAttackError {
    None,
    NotInRange,
    NotFacingTarget,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum SpellFailReason {
    AffectingCombat = 0x00,
    AlreadyAtFullHealth = 0x01,
    AlreadyAtFullMana = 0x02,
    AlreadyAtFullPower = 0x03,
    AlreadyBeingTamed = 0x04,
    AlreadyHaveCharm = 0x05,
    AlreadyHaveSummon = 0x06,
    AlreadyOpen = 0x07,
    AuraBounced = 0x08,
    AutotrackInterrupted = 0x09,
    BadImplicitTargets = 0x0A,
    BadTargets = 0x0B,
    CantBeCharmed = 0x0C,
    CantBeDisenchanted = 0x0D,
    CantBeDisenchantedSkill = 0x0E,
    CantBeProspected = 0x0F,
    CantCastOnTapped = 0x10,
    CantDuelWhileInvisible = 0x11,
    CantDuelWhileStealthed = 0x12,
    CantStealth = 0x13,
    CasterAurastate = 0x14,
    CasterDead = 0x15,
    Charmed = 0x16,
    ChestInUse = 0x17,
    Confused = 0x18,
    DontReport = 0x19,
    EquippedItem = 0x1A,
    EquippedItemClass = 0x1B,
    EquippedItemClassMainhand = 0x1C,
    EquippedItemClassOffhand = 0x1D,
    Error = 0x1E,
    Fizzle = 0x1F,
    Fleeing = 0x20,
    FoodLowlevel = 0x21,
    Highlevel = 0x22,
    HungerSatiated = 0x23,
    Immune = 0x24,
    Interrupted = 0x25,
    InterruptedCombat = 0x26,
    ItemAlreadyEnchanted = 0x27,
    ItemGone = 0x28,
    ItemNotFound = 0x29,
    ItemNotReady = 0x2A,
    LevelRequirement = 0x2B,
    LineOfSight = 0x2C,
    Lowlevel = 0x2D,
    LowCastlevel = 0x2E,
    MainhandEmpty = 0x2F,
    Moving = 0x30,
    NeedAmmo = 0x31,
    NeedAmmoPouch = 0x32,
    NeedExoticAmmo = 0x33,
    Nopath = 0x34,
    NotBehind = 0x35,
    NotFishable = 0x36,
    NotFlying = 0x37,
    NotHere = 0x38,
    NotInfront = 0x39,
    NotInControl = 0x3A,
    NotKnown = 0x3B,
    NotMounted = 0x3C,
    NotOnTaxi = 0x3D,
    NotOnTransport = 0x3E,
    NotReady = 0x3F,
    NotShapeshift = 0x40,
    NotStanding = 0x41,
    NotTradeable = 0x42,
    NotTrading = 0x43,
    NotUnsheathed = 0x44,
    NotWhileGhost = 0x45,
    NoAmmo = 0x46,
    NoChargesRemain = 0x47,
    NoChampion = 0x48,
    NoComboPoints = 0x49,
    NoDueling = 0x4A,
    NoEndurance = 0x4B,
    NoFish = 0x4C,
    NoItemsWhileShapeshifted = 0x4D,
    NoMountsAllowed = 0x4E,
    NoPet = 0x4F,
    NoPower = 0x50,
    NothingToDispel = 0x51,
    NothingToSteal = 0x52,
    OnlyAbovewater = 0x53,
    OnlyDaytime = 0x54,
    OnlyIndoors = 0x55,
    OnlyMounted = 0x56,
    OnlyNighttime = 0x57,
    OnlyOutdoors = 0x58,
    OnlyShapeshift = 0x59,
    OnlyStealthed = 0x5A,
    OnlyUnderwater = 0x5B,
    OutOfRange = 0x5C,
    Pacified = 0x5D,
    Possessed = 0x5E,
    Reagents = 0x5F,
    RequiresArea = 0x60,
    RequiresSpellFocus = 0x61,
    Rooted = 0x62,
    Silenced = 0x63,
    SpellInProgress = 0x64,
    SpellLearned = 0x65,
    SpellUnavailable = 0x66,
    Stunned = 0x67,
    TargetsDead = 0x68,
    TargetAffectingCombat = 0x69,
    TargetAurastate = 0x6A,
    TargetDueling = 0x6B,
    TargetEnemy = 0x6C,
    TargetEnraged = 0x6D,
    TargetFriendly = 0x6E,
    TargetInCombat = 0x6F,
    TargetIsPlayer = 0x70,
    TargetIsPlayerControlled = 0x71,
    TargetNotDead = 0x72,
    TargetNotInParty = 0x73,
    TargetNotLooted = 0x74,
    TargetNotPlayer = 0x75,
    TargetNoPockets = 0x76,
    TargetNoWeapons = 0x77,
    TargetUnskinnable = 0x78,
    ThirstSatiated = 0x79,
    TooClose = 0x7A,
    TooManyOfItem = 0x7B,
    TotemCategory = 0x7C,
    Totems = 0x7D,
    TrainingPoints = 0x7E,
    TryAgain = 0x7F,
    UnitNotBehind = 0x80,
    UnitNotInfront = 0x81,
    WrongPetFood = 0x82,
    NotWhileFatigued = 0x83,
    TargetNotInInstance = 0x84,
    NotWhileTrading = 0x85,
    TargetNotInRaid = 0x86,
    DisenchantWhileLooting = 0x87,
    ProspectWhileLooting = 0x88,
    ProspectNeedMore = 0x89,
    TargetFreeforall = 0x8A,
    NoEdibleCorpses = 0x8B,
    OnlyBattlegrounds = 0x8C,
    TargetNotGhost = 0x8D,
    TooManySkills = 0x8E,
    TransformUnusable = 0x8F,
    WrongWeather = 0x90,
    DamageImmune = 0x91,
    PreventedByMechanic = 0x92,
    PlayTime = 0x93,
    Reputation = 0x94,
    MinSkill = 0x95,
    NotInArena = 0x96,
    NotOnShapeshift = 0x97,
    NotOnStealthed = 0x98,
    NotOnDamageImmune = 0x99,
    NotOnMounted = 0x9A,
    TooShallow = 0x9B,
    TargetNotInSanctuary = 0x9C,
    TargetIsTrivial = 0x9D,
    BmOrInvisgod = 0x9E,
    ExpertRidingRequirement = 0x9F,
    ArtisanRidingRequirement = 0xA0,
    NotIdle = 0xA1,
    NotInactive = 0xA2,
    PartialPlaytime = 0xA3,
    NoPlaytime = 0xA4,
    NotInBattleground = 0xA5,
    OnlyInArena = 0xA6,
    TargetLockedToRaidInstance = 0xA7,
    Unknown = 0xA8,
}

#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum QuestFlag {
    StayAlive = 0x00000001,     // Not used currently
    PartyAccept = 0x00000002,   // Show confirmation box to party members upon accepting the quest
    Exploration = 0x00000004,   // Not used currently
    Sharable = 0x00000008,      // Can be shared: Player::CanShareQuest()
    HasCondition = 0x00000010,  // Not used currently
    HideRewardPOI = 0x00000020, // Not used currently: Unsure of content
    Raid = 0x00000040,          // Can be completed while in raid group
    Tbc = 0x00000080,           // Not used currently: Available if Tbc expansion enabled only
    NoMoneyFromXp = 0x00000100, // Not used currently: Experience is not converted to gold at max level
    HiddenRewards = 0x00000200, // Items and money rewarded only sent in SmsgQuestGiverOfferReward
    Tracking = 0x00000400, // These quests are automatically rewarded on quest complete and they will never appear in quest log client side.
    TbcRaces = 0x00000800, // Not used currently
    Daily = 0x00001000,
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum QuestGiverStatus {
    None = 0,
    Unavailable = 1,         // Grey exclamation mark
    Chat = 2,                // Nothing above head but yellow exclamation mark on hover
    Incomplete = 3,          // Grey question mark
    RewardRepeatable = 4,    // Blue question mark
    AvailableRepeatable = 5, // Blue exclamation mark
    Available = 6,           // Yellow exclamation mark
    RewardHideOnMiniMap = 7, // Yellow question mark, no yellow dot on minimap
    Reward = 8,              // Yellow question mark, yellow dot on minimap
}

#[allow(dead_code)]
#[repr(u32)]
#[derive(N, Copy, Clone, PartialEq, Debug)]
pub enum PlayerQuestStatus {
    InProgress = 0,
    ObjectivesCompleted = 1,
    TurnedIn = 2,
    Failed = 3,
    NotStarted = 4, // Player has never taken the quest
}

// Sent in update fields
#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum QuestSlotState {
    Completed = 1,
    Failed = 2,
}

pub const NPC_TEXT_TEXT_COUNT: usize = 8;
pub const NPC_TEXT_EMOTE_COUNT: usize = 3;

pub const QUEST_EMOTE_COUNT: usize = 4;
pub const MAX_QUEST_REWARDS_COUNT: usize = 4;
pub const MAX_QUEST_CHOICE_REWARDS_COUNT: usize = 6;
pub const MAX_QUEST_REWARDS_REPUT_COUNT: usize = 5;
pub const MAX_QUEST_OBJECTIVES_COUNT: usize = 4;
pub const MAX_QUESTS_IN_LOG: usize = 25;

#[allow(dead_code)]
#[derive(Debug)]
pub enum QuestStartError {
    FailedRequirement = 0,     // this is default case
    TooLowLevel = 1,           // You are not high enough level for that quest.
    WrongRace = 6,             // That quest is not available to your race.
    AlreadyDone = 7,           // You have completed that quest.
    OnlyOneTimed = 12,         // You can only be on one timed quest at a time.
    AlreadyOn = 13,            // You are already on that quest.
    Expansion = 16,            // This quest requires an expansion enabled account.
    AlreadyOn2 = 18,           // You are already on that quest.
    MissingItems = 21,         // You don't have the required items with you. Check storage.
    NotEnoughMoney = 23,       // You don't have enough money for that quest.
    DailyQuestsRemaining = 26, // You have already completed 10 daily quests today.
    Tired = 27,                // You cannot complete quests once you have reached tired time.
}

#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MovementFlag {
    Forward = 0x00000001,
    Backward = 0x00000002,
    StrafeLeft = 0x00000004,
    StrafeRight = 0x00000008,
    Left = 0x00000010,  // Turning left
    Right = 0x00000020, // Turning right
    PitchUp = 0x00000040,
    PitchDown = 0x00000080,
    Walking = 0x00000100,          // Walking
    Ontransport = 0x00000200,      // Used for flying on some creatures
    DisableGravity = 0x00000400, // Unit appears suspended (swimming in the air) instead of falling
    Root = 0x00000800,           // Must not be set along with MovementflagMaskMoving
    JumpingOrFalling = 0x00001000, // Tc: MovementflagFalling // damage dealt on that type of falling
    FallingFar = 0x00002000, // Falling under the map boundaries (clientside the camera will remain at the boundaries and let see the character fall below)
    PendingStop = 0x00004000,
    PendingStrafeStop = 0x00008000,
    PendingForward = 0x00010000,
    PendingBackward = 0x00020000,
    PendingStrafeLeft = 0x00040000,
    PendingStrafeRight = 0x00080000,
    PendingRoot = 0x00100000,
    Swimming = 0x00200000,  // appears with fly flag also
    Ascending = 0x00400000, // press "space" when flying or swimming
    Descending = 0x00800000,
    CanFly = 0x01000000, // Player can fly. Seems to work on some degree on creatures.
    PlayerFlying = 0x02000000, // Tc MovementflagFlying
    SplineElevation = 0x04000000, // used for flight paths
    SplineEnabled = 0x08000000, // used for flight paths
    Waterwalking = 0x10000000, // prevent unit from falling through water
    FallingSlow = 0x20000000, // active rogue safe fall spell (passive)
    Hover = 0x40000000,  // hover, cannot jump
}

#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum SplineFlag {
    Done = 0x00000001,
    Falling = 0x00000002, // Affects elevation computation
    Unknown3 = 0x00000004,
    Unknown4 = 0x00000008,
    Unknown5 = 0x00000010,
    Unknown6 = 0x00000020,
    Unknown7 = 0x00000040,
    Unknown8 = 0x00000080,
    Runmode = 0x00000100,
    Flying = 0x00000200, // Smooth movement(Catmullrom interpolation mode), flying animation
    NoSpline = 0x00000400,
    Unknown12 = 0x00000800,
    Unknown13 = 0x00001000,
    Unknown14 = 0x00002000,
    Unknown15 = 0x00004000,
    Unknown16 = 0x00008000,
    FinalPoint = 0x00010000,
    FinalTarget = 0x00020000,
    FinalAngle = 0x00040000,
    Unknown19 = 0x00080000,  // exists, but unknown what it does
    Cyclic = 0x00100000,     // Movement by cycled spline
    EnterCycle = 0x00200000, // Appears with cyclic flag in monster move packet, erases first spline vertex after first cycle done
    Frozen = 0x00400000,     // Will never arrive
    Unknown23 = 0x00800000,
    Unknown24 = 0x01000000,
    Unknown25 = 0x02000000, // exists, but unknown what it does
    Unknown26 = 0x04000000,
    Unknown27 = 0x08000000,
    Unknown28 = 0x10000000,
    Unknown29 = 0x20000000,
    Unknown30 = 0x40000000,
    Unknown31 = 0x80000000,
}

pub const CREATURE_AGGRO_DISTANCE_MIN: f32 = 5.;
pub const CREATURE_AGGRO_DISTANCE_MAX: f32 = 45.;
pub const CREATURE_AGGRO_DISTANCE_AT_SAME_LEVEL: f32 = 20.;
pub const MAX_LEVEL_DIFFERENCE_FOR_AGGRO: i32 = -25;
// TODO: To properly implement leashing, see https://github.com/vmangos/core/issues/793
pub const CREATURE_LEASH_DISTANCE: f32 = 100.;
// Max distance that the target is allowed to move before the chasing entity has to update its
// destination
pub const MAX_CHASE_LEEWAY: f32 = 0.5;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum UnitAttribute {
    Strength = 0,
    Agility = 1,
    Stamina = 2,
    Intellect = 3,
    Spirit = 4,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpellSchool {
    Normal = 0, // Physical
    Holy = 1,
    Fire = 2,
    Nature = 3,
    Frost = 4,
    Shadow = 5,
    Arcane = 6,
}

pub const BASE_ATTACK_TIME: Duration = Duration::from_millis(2000);
pub const BASE_DAMAGE: f32 = 2.;

#[allow(dead_code)]
#[derive(N, Debug, Clone, Copy)]
pub enum Expansion {
    Vanilla,
    Tbc,
}
pub const MAX_EXPANSION: usize = 2;

#[allow(dead_code)]
#[derive(N, Copy, Clone)]
pub enum CreatureRank {
    Normal = 0,
    Elite = 1,
    RareElite = 2,
    WorldBoss = 3,
    Rare = 4,
    Unknown = 5,
}

impl CreatureRank {
    pub fn is_elite(&self) -> bool {
        match self {
            CreatureRank::Normal => false,
            CreatureRank::Elite => true,
            CreatureRank::RareElite => true,
            CreatureRank::WorldBoss => true,
            CreatureRank::Rare => false,
            CreatureRank::Unknown => false,
        }
    }
}

// From DBC: "Friendly" faction template
pub const FRIENDLY_FACTION_TEMPLATE_ID: u32 = 35;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LootType {
    None = 0, // In case of error
    Corpse = 1,
    Pickpocketing = 2,
    Fishing = 3,
    Disenchanting = 4,
    // The next ones are ignored by the client
    Skinning = 6,     // Unsupported by the client, send LOOT_PICKPOCKETING instead
    Prospecting = 7,  // Unsupported by the client, send LOOT_PICKPOCKETING instead
    FishingHole = 20, // Unsupported by the client, send LOOT_FISHING instead
    FishingFail = 21, // Unsupported by the client, send LOOT_FISHING instead
    Insignia = 22,    // Unsupported by the client, send LOOT_CORPSE instead
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum LootSlotType {
    Normal = 0,              // can be looted
    ViewOnly = 1,            // can only view (ignore any loot attempts)
    MasterLooter = 2,        // can be looted only master looter (error message)
    MissingRequirements = 3, // can't be looted (error message about missing reqs)
    Owner = 4,               // ignore binding confirmation and etc, for single player looting
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum LootError {
    DidntKill = 0,             // You don't have permission to loot that corpse.
    TooFar = 4,                // You are too far away to loot that corpse.
    BadFacing = 5,             // You must be facing the corpse to loot it.
    Locked = 6,                // Someone is already looting that corpse.
    NotStanding = 8,           // You need to be standing up to loot something!
    Stunned = 9,               // You can't loot anything while stunned!
    PlayerNotFound = 10,       // Player not found
    PlayTimeExceeded = 11,     // Maximum play time exceeded
    MasterInvFull = 12,        // That player's inventory is full
    MasterUniqueItem = 13,     // Player has too many of that item already
    MasterOther = 14,          // Can't assign item to that player
    AlreadyPickpocketed = 15,  // Your target has already had its pockets picked
    NotWhileShapeshifted = 16, // You can't do that while shapeshifted.
}

#[allow(dead_code)]
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum AttributeModifier {
    StatStrength,
    StatAgility,
    StatStamina,
    StatIntellect,
    StatSpirit,
    Health,
    Mana,
    Rage,
    Focus,
    Energy,
    Happiness,
    Armor,
    ResistanceHoly,
    ResistanceFire,
    ResistanceNature,
    ResistanceFrost,
    ResistanceShadow,
    ResistanceArcane,
    AttackPower,
    AttackPowerRanged,
    DamageMainHand,
    DamageOffHand,
    DamageRanged,
    Max,
}

// An attribute total value is calculated from the following formula:
// ((BaseValue * BasePercent) + totalValue) * TotalPercent
#[allow(dead_code)]
pub enum AttributeModifierType {
    BaseValue,
    BasePercent,
    TotalValue,
    TotalPercent,
    Max,
}

#[allow(dead_code)]
#[bitflags]
#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum GameObjectDynamicLowFlags {
    Activate = 0x01,   // enables interaction with GO
    Animate = 0x02,    // possibly more distinct animation of GO
    NoInteract = 0x04, // appears to disable interaction (not fully verified)
    Sparkle = 0x08,    // makes GO sparkle
}

#[allow(dead_code)]
#[derive(N, Clone, Copy)]
pub enum GameObjectType {
    Door = 0,
    Button = 1,
    QuestGiver = 2,
    Chest = 3,
    Binder = 4,
    Generic = 5,
    Trap = 6,
    Chair = 7,
    SpellFocus = 8,
    Text = 9,
    Goober = 10,
    Transport = 11,
    AreaDamage = 12,
    Camera = 13,
    MapObject = 14,
    MoTransport = 15,
    DuelArbiter = 16,
    FishingNode = 17,
    SummoningRitual = 18,
    MailBox = 19,
    AuctionHouse = 20,
    GuardPost = 21,
    SpellCaster = 22,
    MeetingStone = 23,
    FlagStand = 24,
    FishingHole = 25,
    FlagDrop = 26,
    MiniGame = 27,
    LotteryKiosk = 28,
    CapturePoint = 29,
    AuraGenerator = 30,
    DungeonDifficulty = 31,
    BarberChair = 32,
    DestructibleBuilding = 33,
    GuildBank = 34,
}

#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum SpellCastTargetFlags {
    // Self = 0x00000000,
    Unused1 = 0x00000001, // not used in any spells (can be set dynamically)
    Unit = 0x00000002,    // pguid
    Unused2 = 0x00000004, // not used in any spells (can be set dynamically)
    Unused3 = 0x00000008, // not used in any spells (can be set dynamically)
    Item = 0x00000010,    // pguid
    SourceLocation = 0x00000020, // 3 float
    DestLocation = 0x00000040, // 3 float
    ObjectUnk = 0x00000080, // used in 7 spells only
    UnitUnk = 0x00000100, // looks like self target (389 spells)
    PvpCorpse = 0x00000200, // pguid
    UnitCorpse = 0x00000400, // 10 spells (gathering professions)
    Object = 0x00000800,  // pguid, 0 spells
    TradeItem = 0x00001000, // pguid, 0 spells
    String = 0x00002000,  // string, 0 spells
    GameobjectItem = 0x00004000, // 199 spells, opening object/lock
    Corpse = 0x00008000,  // pguid, resurrection spells
    Unk2 = 0x00010000,    // pguid, not used in any spells (can be set dynamically)
}

#[repr(u32)]
pub enum RemarkableSpells {
    Bind = 3286, // Cast by innkeepers when players set their bind point
}

#[allow(dead_code)]
#[derive(Copy, Clone, N, Debug)]
pub enum TrainerType {
    Class = 0,
    Mount = 1,
    Tradeskill = 2,
    Pet = 3,
}

#[allow(dead_code)]
#[derive(Copy, Clone, N, Debug)]
pub enum TrainerSpellState {
    Green = 0,
    Red = 1,
    Gray = 2,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum BuyFailedReason {
    CantFindItem = 0,
    ItemAlreadySold = 1,
    NotEnoughtMoney = 2,
    SellerDontLikeYou = 4,
    DistanceTooFar = 5,
    ItemSoldOut = 7,
    CantCarryMore = 8,
    RankRequire = 11,
    ReputationRequire = 12,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum SellFailedReason {
    CantFindItem = 1,
    CantSellItem = 2,       // "Merchant doesn't like that item"
    CantFindVendor = 3,     // "Merchant doesn't like you"
    YouDontOwnThatItem = 4, // "You don't own that item"
    Unk = 5,                // <nothing appears>
    OnlyEmptyBag = 6,       // "You an only do that with empty bags"
}
