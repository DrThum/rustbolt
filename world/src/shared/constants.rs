use enumflags2::bitflags;
use enumn::N;

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
#[derive(Clone, Copy, N, PartialEq, Eq)]
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
