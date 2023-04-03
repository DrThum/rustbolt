#[allow(dead_code)]
pub enum ObjectFields {
    ObjectFieldGuid = 0x0000,    // Size: 2, Type: Long, Flags: Public
    ObjectFieldType = 0x0002,    // Size: 1, Type: Int, Flags: Public
    ObjectFieldEntry = 0x0003,   // Size: 1, Type: Int, Flags: Public
    ObjectFieldScaleX = 0x0004,  // Size: 1, Type: Float, Flags: Public
    ObjectFieldPadding = 0x0005, // Size: 1, Type: Int, Flags: None
}

impl Into<usize> for ObjectFields {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
const OBJECT_END: isize = 0x0006;

#[allow(dead_code)]
pub enum ItemFields {
    ItemFieldOwner = OBJECT_END + 0x0000, // Size: 2, Type: Long, Flags: Public
    ItemFieldContained = OBJECT_END + 0x0002, // Size: 2, Type: Long, Flags: Public
    ItemFieldCreator = OBJECT_END + 0x0004, // Size: 2, Type: Long, Flags: Public
    ItemFieldGiftcreator = OBJECT_END + 0x0006, // Size: 2, Type: Long, Flags: Public
    ItemFieldStackCount = OBJECT_END + 0x0008, // Size: 1, Type: Int, Flags: OwnerOnly, Unk2
    ItemFieldDuration = OBJECT_END + 0x0009, // Size: 1, Type: Int, Flags: OwnerOnly, Unk2
    ItemFieldSpellCharges = OBJECT_END + 0x000A, // Size: 5, Type: Int, Flags: OwnerOnly, Unk2
    ItemFieldFlags = OBJECT_END + 0x000F, // Size: 1, Type: Int, Flags: Public
    ItemFieldEnchantment1_1 = OBJECT_END + 0x0010, // Size: 33, Type: Int, Flags: Public
    ItemFieldPropertySeed = OBJECT_END + 0x0031, // Size: 1, Type: Int, Flags: Public
    ItemFieldRandomPropertiesId = OBJECT_END + 0x0032, // Size: 1, Type: Int, Flags: Public
    ItemFieldItemTextId = OBJECT_END + 0x0033, // Size: 1, Type: Int, Flags: OwnerOnly
    ItemFieldDurability = OBJECT_END + 0x0034, // Size: 1, Type: Int, Flags: OwnerOnly, Unk2
    ItemFieldMaxdurability = OBJECT_END + 0x0035, // Size: 1, Type: Int, Flags: OwnerOnly, Unk2
}

impl Into<usize> for ItemFields {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
const ITEM_END: isize = OBJECT_END + 0x0036;

#[allow(dead_code)]
pub enum ContainerFields {
    ContainerFieldNumSlots = ITEM_END + 0x0000, // Size: 1, Type: Int, Flags: Public
    ContainerAlignPad = ITEM_END + 0x0001,      // Size: 1, Type: Bytes, Flags: None
    ContainerFieldSlot1 = ITEM_END + 0x0002,    // Size: 72, Type: Long, Flags: Public
}

impl Into<usize> for ContainerFields {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
const CONTAINER_END: isize = ITEM_END + 0x004A;

#[allow(dead_code)]
pub enum UnitFields {
    UnitFieldCharm = OBJECT_END + 0x0000, // Size: 2, Type: Long, Flags: Public
    UnitFieldSummon = OBJECT_END + 0x0002, // Size: 2, Type: Long, Flags: Public
    UnitFieldCharmedby = OBJECT_END + 0x0004, // Size: 2, Type: Long, Flags: Public
    UnitFieldSummonedby = OBJECT_END + 0x0006, // Size: 2, Type: Long, Flags: Public
    UnitFieldCreatedby = OBJECT_END + 0x0008, // Size: 2, Type: Long, Flags: Public
    UnitFieldTarget = OBJECT_END + 0x000A, // Size: 2, Type: Long, Flags: Public
    UnitFieldPersuaded = OBJECT_END + 0x000C, // Size: 2, Type: Long, Flags: Public
    UnitFieldChannelObject = OBJECT_END + 0x000E, // Size: 2, Type: Long, Flags: Public
    UnitFieldHealth = OBJECT_END + 0x0010, // Size: 1, Type: Int, Flags: Dynamic
    UnitFieldPower1 = OBJECT_END + 0x0011, // Size: 1, Type: Int, Flags: Public
    UnitFieldPower2 = OBJECT_END + 0x0012, // Size: 1, Type: Int, Flags: Public
    UnitFieldPower3 = OBJECT_END + 0x0013, // Size: 1, Type: Int, Flags: Public
    UnitFieldPower4 = OBJECT_END + 0x0014, // Size: 1, Type: Int, Flags: Public
    UnitFieldPower5 = OBJECT_END + 0x0015, // Size: 1, Type: Int, Flags: Public
    UnitFieldMaxhealth = OBJECT_END + 0x0016, // Size: 1, Type: Int, Flags: Dynamic
    UnitFieldMaxpower1 = OBJECT_END + 0x0017, // Size: 1, Type: Int, Flags: Public
    UnitFieldMaxpower2 = OBJECT_END + 0x0018, // Size: 1, Type: Int, Flags: Public
    UnitFieldMaxpower3 = OBJECT_END + 0x0019, // Size: 1, Type: Int, Flags: Public
    UnitFieldMaxpower4 = OBJECT_END + 0x001A, // Size: 1, Type: Int, Flags: Public
    UnitFieldMaxpower5 = OBJECT_END + 0x001B, // Size: 1, Type: Int, Flags: Public
    UnitFieldLevel = OBJECT_END + 0x001C, // Size: 1, Type: Int, Flags: Public
    UnitFieldFactiontemplate = OBJECT_END + 0x001D, // Size: 1, Type: Int, Flags: Public
    UnitFieldBytes0 = OBJECT_END + 0x001E, // Size: 1, Type: Bytes, Flags: Public
    UnitVirtualItemSlotDisplay = OBJECT_END + 0x001F, // Size: 3, Type: Int, Flags: Public
    UnitVirtualItemInfo = OBJECT_END + 0x0022, // Size: 6, Type: Bytes, Flags: Public
    UnitFieldFlags = OBJECT_END + 0x0028, // Size: 1, Type: Int, Flags: Public
    UnitFieldFlags2 = OBJECT_END + 0x0029, // Size: 1, Type: Int, Flags: Public
    UnitFieldAura = OBJECT_END + 0x002A,  // Size: 56, Type: Int, Flags: Public
    UnitFieldAuraflags = OBJECT_END + 0x0062, // Size: 14, Type: Bytes, Flags: Public
    UnitFieldAuralevels = OBJECT_END + 0x0070, // Size: 14, Type: Bytes, Flags: Public
    UnitFieldAuraapplications = OBJECT_END + 0x007E, // Size: 14, Type: Bytes, Flags: Public
    UnitFieldAurastate = OBJECT_END + 0x008C, // Size: 1, Type: Int, Flags: Public
    UnitFieldBaseattacktime = OBJECT_END + 0x008D, // Size: 2, Type: Int, Flags: Public
    UnitFieldRangedattacktime = OBJECT_END + 0x008F, // Size: 1, Type: Int, Flags: Private
    UnitFieldBoundingradius = OBJECT_END + 0x0090, // Size: 1, Type: Float, Flags: Public
    UnitFieldCombatreach = OBJECT_END + 0x0091, // Size: 1, Type: Float, Flags: Public
    UnitFieldDisplayid = OBJECT_END + 0x0092, // Size: 1, Type: Int, Flags: Public
    UnitFieldNativedisplayid = OBJECT_END + 0x0093, // Size: 1, Type: Int, Flags: Public
    UnitFieldMountdisplayid = OBJECT_END + 0x0094, // Size: 1, Type: Int, Flags: Public
    UnitFieldMindamage = OBJECT_END + 0x0095, // Size: 1, Type: Float, Flags: Private, OwnerOnly, Unk3
    UnitFieldMaxdamage = OBJECT_END + 0x0096, // Size: 1, Type: Float, Flags: Private, OwnerOnly, Unk3
    UnitFieldMinoffhanddamage = OBJECT_END + 0x0097, // Size: 1, Type: Float, Flags: Private, OwnerOnly, Unk3
    UnitFieldMaxoffhanddamage = OBJECT_END + 0x0098, // Size: 1, Type: Float, Flags: Private, OwnerOnly, Unk3
    UnitFieldBytes1 = OBJECT_END + 0x0099,           // Size: 1, Type: Bytes, Flags: Public
    UnitFieldPetnumber = OBJECT_END + 0x009A,        // Size: 1, Type: Int, Flags: Public
    UnitFieldPetNameTimestamp = OBJECT_END + 0x009B, // Size: 1, Type: Int, Flags: Public
    UnitFieldPetexperience = OBJECT_END + 0x009C,    // Size: 1, Type: Int, Flags: OwnerOnly
    UnitFieldPetnextlevelexp = OBJECT_END + 0x009D,  // Size: 1, Type: Int, Flags: OwnerOnly
    UnitDynamicFlags = OBJECT_END + 0x009E,          // Size: 1, Type: Int, Flags: Dynamic
    UnitChannelSpell = OBJECT_END + 0x009F,          // Size: 1, Type: Int, Flags: Public
    UnitModCastSpeed = OBJECT_END + 0x00A0,          // Size: 1, Type: Float, Flags: Public
    UnitCreatedBySpell = OBJECT_END + 0x00A1,        // Size: 1, Type: Int, Flags: Public
    UnitNpcFlags = OBJECT_END + 0x00A2,              // Size: 1, Type: Int, Flags: Dynamic
    UnitNpcEmotestate = OBJECT_END + 0x00A3,         // Size: 1, Type: Int, Flags: Public
    UnitTrainingPoints = OBJECT_END + 0x00A4,        // Size: 1, Type: TwoShort, Flags: OwnerOnly
    UnitFieldStat0 = OBJECT_END + 0x00A5, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldStat1 = OBJECT_END + 0x00A6, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldStat2 = OBJECT_END + 0x00A7, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldStat3 = OBJECT_END + 0x00A8, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldStat4 = OBJECT_END + 0x00A9, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    PlayerFieldPosstat0 = OBJECT_END + 0x00Aa, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldPosstat1 = OBJECT_END + 0x00Ab, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldPosstat2 = OBJECT_END + 0x00Ac, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldPosstat3 = OBJECT_END + 0x00Ad, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    PlayerFieldPosstat4 = OBJECT_END + 0x00Ae, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    PlayerFieldNegstat0 = OBJECT_END + 0x00Af, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldNegstat1 = OBJECT_END + 0x00B0, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldNegstat2 = OBJECT_END + 0x00B1, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldNegstat3 = OBJECT_END + 0x00B2, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    PlayerFieldNegstat4 = OBJECT_END + 0x00B3, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldResistances = OBJECT_END + 0x00B4, // Size: 7, Type: Int, Flags: Private, OwnerOnly, Unk3
    PlayerFieldResistancebuffmodspositive = OBJECT_END + 0x00Bb, // Size: 7, Type: Int, Flags: Private, OwnerOnly
    PlayerFieldResistancebuffmodsnegative = OBJECT_END + 0x00C2, // Size: 7, Type: Int, Flags: Private, OwnerOnly
    UnitFieldBaseMana = OBJECT_END + 0x00C9, // Size: 1, Type: Int, Flags: Public
    UnitFieldBaseHealth = OBJECT_END + 0x00Ca, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldBytes2 = OBJECT_END + 0x00Cb,   // Size: 1, Type: Bytes, Flags: Public
    UnitFieldAttackPower = OBJECT_END + 0x00Cc, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldAttackPowerMods = OBJECT_END + 0x00Cd, // Size: 1, Type: TwoShort, Flags: Private, OwnerOnly
    UnitFieldAttackPowerMultiplier = OBJECT_END + 0x00Ce, // Size: 1, Type: Float, Flags: Private, OwnerOnly
    UnitFieldRangedAttackPower = OBJECT_END + 0x00Cf, // Size: 1, Type: Int, Flags: Private, OwnerOnly
    UnitFieldRangedAttackPowerMods = OBJECT_END + 0x00D0, // Size: 1, Type: TwoShort, Flags: Private, OwnerOnly
    UnitFieldRangedAttackPowerMultiplier = OBJECT_END + 0x00D1, // Size: 1, Type: Float, Flags: Private, OwnerOnly
    UnitFieldMinrangeddamage = OBJECT_END + 0x00D2, // Size: 1, Type: Float, Flags: Private, OwnerOnly
    UnitFieldMaxrangeddamage = OBJECT_END + 0x00D3, // Size: 1, Type: Float, Flags: Private, OwnerOnly
    UnitFieldPowerCostModifier = OBJECT_END + 0x00D4, // Size: 7, Type: Int, Flags: Private, OwnerOnly
    UnitFieldPowerCostMultiplier = OBJECT_END + 0x00Db, // Size: 7, Type: Float, Flags: Private, OwnerOnly
    UnitFieldMaxhealthmodifier = OBJECT_END + 0x00E2, // Size: 1, Type: Float, Flags: Private, OwnerOnly
    UnitFieldPadding = OBJECT_END + 0x00E3,           // Size: 1, Type: Int, Flags: None

    PlayerDuelArbiter = UNIT_END + 0x0000, // Size: 2, Type: Long, Flags: Public
    PlayerFlags = UNIT_END + 0x0002,       // Size: 1, Type: Int, Flags: Public
    PlayerGuildid = UNIT_END + 0x0003,     // Size: 1, Type: Int, Flags: Public
    PlayerGuildrank = UNIT_END + 0x0004,   // Size: 1, Type: Int, Flags: Public
    PlayerBytes = UNIT_END + 0x0005,       // Size: 1, Type: Bytes, Flags: Public
    PlayerBytes2 = UNIT_END + 0x0006,      // Size: 1, Type: Bytes, Flags: Public
    PlayerBytes3 = UNIT_END + 0x0007,      // Size: 1, Type: Bytes, Flags: Public
    PlayerDuelTeam = UNIT_END + 0x0008,    // Size: 1, Type: Int, Flags: Public
    PlayerGuildTimestamp = UNIT_END + 0x0009, // Size: 1, Type: Int, Flags: Public
    PlayerQuestLog1_1 = UNIT_END + 0x000A, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog1_2 = UNIT_END + 0x000B, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog1_3 = UNIT_END + 0x000C, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog1_4 = UNIT_END + 0x000D, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog2_1 = UNIT_END + 0x000E, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog2_2 = UNIT_END + 0x000F, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog2_3 = UNIT_END + 0x0010, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog2_4 = UNIT_END + 0x0011, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog3_1 = UNIT_END + 0x0012, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog3_2 = UNIT_END + 0x0013, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog3_3 = UNIT_END + 0x0014, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog3_4 = UNIT_END + 0x0015, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog4_1 = UNIT_END + 0x0016, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog4_2 = UNIT_END + 0x0017, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog4_3 = UNIT_END + 0x0018, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog4_4 = UNIT_END + 0x0019, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog5_1 = UNIT_END + 0x001A, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog5_2 = UNIT_END + 0x001B, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog5_3 = UNIT_END + 0x001C, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog5_4 = UNIT_END + 0x001D, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog6_1 = UNIT_END + 0x001E, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog6_2 = UNIT_END + 0x001F, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog6_3 = UNIT_END + 0x0020, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog6_4 = UNIT_END + 0x0021, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog7_1 = UNIT_END + 0x0022, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog7_2 = UNIT_END + 0x0023, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog7_3 = UNIT_END + 0x0024, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog7_4 = UNIT_END + 0x0025, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog8_1 = UNIT_END + 0x0026, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog8_2 = UNIT_END + 0x0027, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog8_3 = UNIT_END + 0x0028, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog8_4 = UNIT_END + 0x0029, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog9_1 = UNIT_END + 0x002A, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog9_2 = UNIT_END + 0x002B, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog9_3 = UNIT_END + 0x002C, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog9_4 = UNIT_END + 0x002D, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog10_1 = UNIT_END + 0x002E, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog10_2 = UNIT_END + 0x002F, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog10_3 = UNIT_END + 0x0030, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog10_4 = UNIT_END + 0x0031, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog11_1 = UNIT_END + 0x0032, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog11_2 = UNIT_END + 0x0033, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog11_3 = UNIT_END + 0x0034, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog11_4 = UNIT_END + 0x0035, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog12_1 = UNIT_END + 0x0036, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog12_2 = UNIT_END + 0x0037, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog12_3 = UNIT_END + 0x0038, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog12_4 = UNIT_END + 0x0039, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog13_1 = UNIT_END + 0x003A, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog13_2 = UNIT_END + 0x003B, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog13_3 = UNIT_END + 0x003C, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog13_4 = UNIT_END + 0x003D, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog14_1 = UNIT_END + 0x003E, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog14_2 = UNIT_END + 0x003F, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog14_3 = UNIT_END + 0x0040, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog14_4 = UNIT_END + 0x0041, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog15_1 = UNIT_END + 0x0042, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog15_2 = UNIT_END + 0x0043, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog15_3 = UNIT_END + 0x0044, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog15_4 = UNIT_END + 0x0045, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog16_1 = UNIT_END + 0x0046, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog16_2 = UNIT_END + 0x0047, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog16_3 = UNIT_END + 0x0048, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog16_4 = UNIT_END + 0x0049, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog17_1 = UNIT_END + 0x004A, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog17_2 = UNIT_END + 0x004B, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog17_3 = UNIT_END + 0x004C, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog17_4 = UNIT_END + 0x004D, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog18_1 = UNIT_END + 0x004E, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog18_2 = UNIT_END + 0x004F, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog18_3 = UNIT_END + 0x0050, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog18_4 = UNIT_END + 0x0051, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog19_1 = UNIT_END + 0x0052, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog19_2 = UNIT_END + 0x0053, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog19_3 = UNIT_END + 0x0054, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog19_4 = UNIT_END + 0x0055, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog20_1 = UNIT_END + 0x0056, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog20_2 = UNIT_END + 0x0057, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog20_3 = UNIT_END + 0x0058, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog20_4 = UNIT_END + 0x0059, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog21_1 = UNIT_END + 0x005A, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog21_2 = UNIT_END + 0x005B, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog21_3 = UNIT_END + 0x005C, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog21_4 = UNIT_END + 0x005D, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog22_1 = UNIT_END + 0x005E, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog22_2 = UNIT_END + 0x005F, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog22_3 = UNIT_END + 0x0060, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog22_4 = UNIT_END + 0x0061, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog23_1 = UNIT_END + 0x0062, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog23_2 = UNIT_END + 0x0063, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog23_3 = UNIT_END + 0x0064, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog23_4 = UNIT_END + 0x0065, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog24_1 = UNIT_END + 0x0066, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog24_2 = UNIT_END + 0x0067, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog24_3 = UNIT_END + 0x0068, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog24_4 = UNIT_END + 0x0069, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog25_1 = UNIT_END + 0x006A, // Size: 1, Type: Int, Flags: GroupOnly
    PlayerQuestLog25_2 = UNIT_END + 0x006B, // Size: 1, Type: Int, Flags: Private
    PlayerQuestLog25_3 = UNIT_END + 0x006C, // Size: 1, Type: Bytes, Flags: Private
    PlayerQuestLog25_4 = UNIT_END + 0x006D, // Size: 1, Type: Int, Flags: Private
    PlayerVisibleItem1Creator = UNIT_END + 0x006E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem1_0 = UNIT_END + 0x0070, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem1Properties = UNIT_END + 0x007C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem1Pad = UNIT_END + 0x007D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem2Creator = UNIT_END + 0x007E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem2_0 = UNIT_END + 0x0080, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem2Properties = UNIT_END + 0x008C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem2Pad = UNIT_END + 0x008D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem3Creator = UNIT_END + 0x008E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem3_0 = UNIT_END + 0x0090, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem3Properties = UNIT_END + 0x009C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem3Pad = UNIT_END + 0x009D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem4Creator = UNIT_END + 0x009E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem4_0 = UNIT_END + 0x00A0, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem4Properties = UNIT_END + 0x00Ac, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem4Pad = UNIT_END + 0x00Ad, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem5Creator = UNIT_END + 0x00Ae, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem5_0 = UNIT_END + 0x00B0, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem5Properties = UNIT_END + 0x00Bc, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem5Pad = UNIT_END + 0x00Bd, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem6Creator = UNIT_END + 0x00Be, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem6_0 = UNIT_END + 0x00C0, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem6Properties = UNIT_END + 0x00Cc, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem6Pad = UNIT_END + 0x00Cd, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem7Creator = UNIT_END + 0x00Ce, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem7_0 = UNIT_END + 0x00D0, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem7Properties = UNIT_END + 0x00Dc, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem7Pad = UNIT_END + 0x00Dd, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem8Creator = UNIT_END + 0x00De, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem8_0 = UNIT_END + 0x00E0, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem8Properties = UNIT_END + 0x00Ec, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem8Pad = UNIT_END + 0x00Ed, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem9Creator = UNIT_END + 0x00Ee, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem9_0 = UNIT_END + 0x00F0, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem9Properties = UNIT_END + 0x00Fc, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem9Pad = UNIT_END + 0x00Fd, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem10Creator = UNIT_END + 0x00Fe, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem10_0 = UNIT_END + 0x0100, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem10Properties = UNIT_END + 0x010C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem10Pad = UNIT_END + 0x010D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem11Creator = UNIT_END + 0x010E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem11_0 = UNIT_END + 0x0110, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem11Properties = UNIT_END + 0x011C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem11Pad = UNIT_END + 0x011D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem12Creator = UNIT_END + 0x011E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem12_0 = UNIT_END + 0x0120, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem12Properties = UNIT_END + 0x012C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem12Pad = UNIT_END + 0x012D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem13Creator = UNIT_END + 0x012E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem13_0 = UNIT_END + 0x0130, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem13Properties = UNIT_END + 0x013C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem13Pad = UNIT_END + 0x013D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem14Creator = UNIT_END + 0x013E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem14_0 = UNIT_END + 0x0140, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem14Properties = UNIT_END + 0x014C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem14Pad = UNIT_END + 0x014D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem15Creator = UNIT_END + 0x014E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem15_0 = UNIT_END + 0x0150, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem15Properties = UNIT_END + 0x015C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem15Pad = UNIT_END + 0x015D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem16Creator = UNIT_END + 0x015E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem16_0 = UNIT_END + 0x0160, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem16Properties = UNIT_END + 0x016C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem16Pad = UNIT_END + 0x016D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem17Creator = UNIT_END + 0x016E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem17_0 = UNIT_END + 0x0170, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem17Properties = UNIT_END + 0x017C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem17Pad = UNIT_END + 0x017D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem18Creator = UNIT_END + 0x017E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem18_0 = UNIT_END + 0x0180, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem18Properties = UNIT_END + 0x018C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem18Pad = UNIT_END + 0x018D, // Size: 1, Type: Int, Flags: Public
    PlayerVisibleItem19Creator = UNIT_END + 0x018E, // Size: 2, Type: Long, Flags: Public
    PlayerVisibleItem19_0 = UNIT_END + 0x0190, // Size: 12, Type: Int, Flags: Public
    PlayerVisibleItem19properties = UNIT_END + 0x019C, // Size: 1, Type: TwoShort, Flags: Public
    PlayerVisibleItem19Pad = UNIT_END + 0x019D, // Size: 1, Type: Int, Flags: Public
    PlayerChosenTitle = UNIT_END + 0x019E, // Size: 1, Type: Int, Flags: Public
    PlayerFieldPad0 = UNIT_END + 0x019F,   // Size: 1, Type: Int, Flags: None
    PlayerFieldInvSlotHead = UNIT_END + 0x01A0, // Size: 46, Type: Long, Flags: Private
    PlayerFieldPackSlot1 = UNIT_END + 0x01Ce, // Size: 32, Type: Long, Flags: Private
    PlayerFieldBankSlot1 = UNIT_END + 0x01Ee, // Size: 56, Type: Long, Flags: Private
    PlayerFieldBankbagSlot1 = UNIT_END + 0x0226, // Size: 14, Type: Long, Flags: Private
    PlayerFieldVendorbuybackSlot1 = UNIT_END + 0x0234, // Size: 24, Type: Long, Flags: Private
    PlayerFieldKeyringSlot1 = UNIT_END + 0x024C, // Size: 64, Type: Long, Flags: Private
    PlayerFieldVanitypetSlot1 = UNIT_END + 0x028C, // Size: 36, Type: Long, Flags: Private
    PlayerFarsight = UNIT_END + 0x02B0,    // Size: 2, Type: Long, Flags: Private
    PlayerFieldKnownTitles = UNIT_END + 0x02B2, // Size: 2, Type: Long, Flags: Private
    PlayerXp = UNIT_END + 0x02B4,          // Size: 1, Type: Int, Flags: Private
    PlayerNextLevelXp = UNIT_END + 0x02B5, // Size: 1, Type: Int, Flags: Private
    PlayerSkillInfo1_1 = UNIT_END + 0x02B6, // Size: 384, Type: TwoShort, Flags: Private
    PlayerCharacterPoints1 = UNIT_END + 0x0436, // Size: 1, Type: Int, Flags: Private
    PlayerCharacterPoints2 = UNIT_END + 0x0437, // Size: 1, Type: Int, Flags: Private
    PlayerTrackCreatures = UNIT_END + 0x0438, // Size: 1, Type: Int, Flags: Private
    PlayerTrackResources = UNIT_END + 0x0439, // Size: 1, Type: Int, Flags: Private
    PlayerBlockPercentage = UNIT_END + 0x043A, // Size: 1, Type: Float, Flags: Private
    PlayerDodgePercentage = UNIT_END + 0x043B, // Size: 1, Type: Float, Flags: Private
    PlayerParryPercentage = UNIT_END + 0x043C, // Size: 1, Type: Float, Flags: Private
    PlayerExpertise = UNIT_END + 0x043D,   // Size: 1, Type: Int, Flags: Private
    PlayerOffhandExpertise = UNIT_END + 0x043E, // Size: 1, Type: Int, Flags: Private
    PlayerCritPercentage = UNIT_END + 0x043F, // Size: 1, Type: Float, Flags: Private
    PlayerRangedCritPercentage = UNIT_END + 0x0440, // Size: 1, Type: Float, Flags: Private
    PlayerOffhandCritPercentage = UNIT_END + 0x0441, // Size: 1, Type: Float, Flags: Private
    PlayerSpellCritPercentage1 = UNIT_END + 0x0442, // Size: 7, Type: Float, Flags: Private
    PlayerShieldBlock = UNIT_END + 0x0449, // Size: 1, Type: Int, Flags: Private
    PlayerExploredZones1 = UNIT_END + 0x044A, // Size: 128, Type: Bytes, Flags: Private
    PlayerRestStateExperience = UNIT_END + 0x04Ca, // Size: 1, Type: Int, Flags: Private
    PlayerFieldCoinage = UNIT_END + 0x04Cb, // Size: 1, Type: Int, Flags: Private
    PlayerFieldModDamageDonePos = UNIT_END + 0x04Cc, // Size: 7, Type: Int, Flags: Private
    PlayerFieldModDamageDoneNeg = UNIT_END + 0x04D3, // Size: 7, Type: Int, Flags: Private
    PlayerFieldModDamageDonePct = UNIT_END + 0x04Da, // Size: 7, Type: Int, Flags: Private
    PlayerFieldModHealingDonePos = UNIT_END + 0x04E1, // Size: 1, Type: Int, Flags: Private
    PlayerFieldModTargetResistance = UNIT_END + 0x04E2, // Size: 1, Type: Int, Flags: Private
    PlayerFieldModTargetPhysicalResistance = UNIT_END + 0x04E3, // Size: 1, Type: Int, Flags: Private
    PlayerFieldBytes = UNIT_END + 0x04E4, // Size: 1, Type: Bytes, Flags: Private
    PlayerAmmoId = UNIT_END + 0x04E5,     // Size: 1, Type: Int, Flags: Private
    PlayerSelfResSpell = UNIT_END + 0x04E6, // Size: 1, Type: Int, Flags: Private
    PlayerFieldPvpMedals = UNIT_END + 0x04E7, // Size: 1, Type: Int, Flags: Private
    PlayerFieldBuybackPrice1 = UNIT_END + 0x04E8, // Size: 12, Type: Int, Flags: Private
    PlayerFieldBuybackTimestamp1 = UNIT_END + 0x04F4, // Size: 12, Type: Int, Flags: Private
    PlayerFieldKills = UNIT_END + 0x0500, // Size: 1, Type: TwoShort, Flags: Private
    PlayerFieldTodayContribution = UNIT_END + 0x0501, // Size: 1, Type: Int, Flags: Private
    PlayerFieldYesterdayContribution = UNIT_END + 0x0502, // Size: 1, Type: Int, Flags: Private
    PlayerFieldLifetimeHonorableKills = UNIT_END + 0x0503, // Size: 1, Type: Int, Flags: Private
    PlayerFieldBytes2 = UNIT_END + 0x0504, // Size: 1, Type: Bytes, Flags: Private
    PlayerFieldWatchedFactionIndex = UNIT_END + 0x0505, // Size: 1, Type: Int, Flags: Private
    PlayerFieldCombatRating1 = UNIT_END + 0x0506, // Size: 24, Type: Int, Flags: Private
    PlayerFieldArenaTeamInfo1_1 = UNIT_END + 0x051E, // Size: 18, Type: Int, Flags: Private
    PlayerFieldHonorCurrency = UNIT_END + 0x0530, // Size: 1, Type: Int, Flags: Private
    PlayerFieldArenaCurrency = UNIT_END + 0x0531, // Size: 1, Type: Int, Flags: Private
    PlayerFieldModManaRegen = UNIT_END + 0x0532, // Size: 1, Type: Float, Flags: Private
    PlayerFieldModManaRegenInterrupt = UNIT_END + 0x0533, // Size: 1, Type: Float, Flags: Private
    PlayerFieldMaxLevel = UNIT_END + 0x0534, // Size: 1, Type: Int, Flags: Private
    PlayerFieldDailyQuests1 = UNIT_END + 0x0535, // Size: 25, Type: Int, Flags: Private
}

impl Into<usize> for UnitFields {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
const UNIT_END: isize = OBJECT_END + 0x00E4;
#[allow(dead_code)]
const PLAYER_END: isize = UNIT_END + 0x054E;
#[allow(dead_code)]
pub const MAX_PLAYER_VISIBLE_ITEM_OFFSET: u32 = 16;

#[allow(dead_code)]
pub enum GameObjectFields {
    ObjectFieldCreatedBy = OBJECT_END + 0x0000, // Size: 2, Type: Long, Flags: Public
    GameObjectDisplayid = OBJECT_END + 0x0002,  // Size: 1, Type: Int, Flags: Public
    GameObjectFlags = OBJECT_END + 0x0003,      // Size: 1, Type: Int, Flags: Public
    GameObjectRotation = OBJECT_END + 0x0004,   // Size: 4, Type: Float, Flags: Public
    GameObjectState = OBJECT_END + 0x0008,      // Size: 1, Type: Int, Flags: Public
    GameObjectPosX = OBJECT_END + 0x0009,       // Size: 1, Type: Float, Flags: Public
    GameObjectPosY = OBJECT_END + 0x000A,       // Size: 1, Type: Float, Flags: Public
    GameObjectPosZ = OBJECT_END + 0x000B,       // Size: 1, Type: Float, Flags: Public
    GameObjectFacing = OBJECT_END + 0x000C,     // Size: 1, Type: Float, Flags: Public
    GameObjectDynFlags = OBJECT_END + 0x000D,   // Size: 1, Type: Int, Flags: Dynamic
    GameObjectFaction = OBJECT_END + 0x000E,    // Size: 1, Type: Int, Flags: Public
    GameObjectTypeId = OBJECT_END + 0x000F,     // Size: 1, Type: Int, Flags: Public
    GameObjectLevel = OBJECT_END + 0x0010,      // Size: 1, Type: Int, Flags: Public
    GameObjectArtkit = OBJECT_END + 0x0011,     // Size: 1, Type: Int, Flags: Public
    GameObjectAnimprogress = OBJECT_END + 0x0012, // Size: 1, Type: Int, Flags: Dynamic
    GameObjectPadding = OBJECT_END + 0x0013,    // Size: 1, Type: Int, Flags: None
}

impl Into<usize> for GameObjectFields {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
const GAME_OBJECT_END: isize = OBJECT_END + 0x0014;

#[allow(dead_code)]
pub enum DynamicObjectFields {
    DynamicObjectCaster = OBJECT_END + 0x0000, // Size: 2, Type: Long, Flags: Public
    DynamicObjectBytes = OBJECT_END + 0x0002,  // Size: 1, Type: Bytes, Flags: Public
    DynamicObjectSpellid = OBJECT_END + 0x0003, // Size: 1, Type: Int, Flags: Public
    DynamicObjectRadius = OBJECT_END + 0x0004, // Size: 1, Type: Float, Flags: Public
    DynamicObjectPosX = OBJECT_END + 0x0005,   // Size: 1, Type: Float, Flags: Public
    DynamicObjectPosY = OBJECT_END + 0x0006,   // Size: 1, Type: Float, Flags: Public
    DynamicObjectPosZ = OBJECT_END + 0x0007,   // Size: 1, Type: Float, Flags: Public
    DynamicObjectFacing = OBJECT_END + 0x0008, // Size: 1, Type: Float, Flags: Public
    DynamicObjectCasttime = OBJECT_END + 0x0009, // Size: 1, Type: Int, Flags: Public
}

impl Into<usize> for DynamicObjectFields {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
const DYNAMIC_OBJECT_END: isize = OBJECT_END + 0x000A;

#[allow(dead_code)]
pub enum CorpseFields {
    CorpseFieldOwner = OBJECT_END + 0x0000, // Size: 2, Type: Long, Flags: Public
    CorpseFieldParty = OBJECT_END + 0x0002, // Size: 2, Type: Long, Flags: Public
    CorpseFieldFacing = OBJECT_END + 0x0004, // Size: 1, Type: Float, Flags: Public
    CorpseFieldPosX = OBJECT_END + 0x0005,  // Size: 1, Type: Float, Flags: Public
    CorpseFieldPosY = OBJECT_END + 0x0006,  // Size: 1, Type: Float, Flags: Public
    CorpseFieldPosZ = OBJECT_END + 0x0007,  // Size: 1, Type: Float, Flags: Public
    CorpseFieldDisplayId = OBJECT_END + 0x0008, // Size: 1, Type: Int, Flags: Public
    CorpseFieldItem = OBJECT_END + 0x0009,  // Size: 19, Type: Int, Flags: Public
    CorpseFieldBytes1 = OBJECT_END + 0x001C, // Size: 1, Type: Bytes, Flags: Public
    CorpseFieldBytes2 = OBJECT_END + 0x001D, // Size: 1, Type: Bytes, Flags: Public
    CorpseFieldGuild = OBJECT_END + 0x001E, // Size: 1, Type: Int, Flags: Public
    CorpseFieldFlags = OBJECT_END + 0x001F, // Size: 1, Type: Int, Flags: Public
    CorpseFieldDynamicFlags = OBJECT_END + 0x0020, // Size: 1, Type: Int, Flags: Dynamic
    CorpseFieldPad = OBJECT_END + 0x0021,   // Size: 1, Type: Int, Flags: None
}

impl Into<usize> for CorpseFields {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
const CORPSE_END: isize = OBJECT_END + 0x0022;
