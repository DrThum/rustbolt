use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::datastore::data_types::GameObjectTemplate;

pub struct GameObjectRepository;

impl GameObjectRepository {
    pub fn load_templates(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<GameObjectTemplate> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(entry) FROM game_object_templates")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn
            .prepare_cached(
                "
            SELECT entry, type, display_id, name, cast_bar_caption, faction, flags, size,
                data0, data1, data2, data3, data4, data5, data6, data7, data8, data9, data10,
                data11, data12, data13, data14, data15, data16, data17, data18, data19, data20,
                data21, data22, data23, min_money_loot, max_money_loot
            FROM game_object_templates
            ORDER BY entry
        ",
            )
            .unwrap();

        let result = stmt
            .query_map([], |row| {
                use GameObjectTemplateColumnIndex::*;

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                let template = GameObjectTemplate {
                    entry: row.get(Entry as usize).unwrap(),
                    go_type: row.get(Type as usize).unwrap(),
                    display_id: row.get(DisplayId as usize).unwrap(),
                    name: row.get(Name as usize).unwrap(),
                    cast_bar_caption: row.get(CastBarCaption as usize).unwrap(),
                    faction: row.get(Faction as usize).unwrap(),
                    flags: row.get(Flags as usize).unwrap(),
                    size: row.get(Size as usize).unwrap(),
                    data0: row.get(Data0 as usize).unwrap(),
                    data1: row.get(Data1 as usize).unwrap(),
                    data2: row.get(Data2 as usize).unwrap(),
                    data3: row.get(Data3 as usize).unwrap(),
                    data4: row.get(Data4 as usize).unwrap(),
                    data5: row.get(Data5 as usize).unwrap(),
                    data6: row.get(Data6 as usize).unwrap(),
                    data7: row.get(Data7 as usize).unwrap(),
                    data8: row.get(Data8 as usize).unwrap(),
                    data9: row.get(Data9 as usize).unwrap(),
                    data10: row.get(Data10 as usize).unwrap(),
                    data11: row.get(Data11 as usize).unwrap(),
                    data12: row.get(Data12 as usize).unwrap(),
                    data13: row.get(Data13 as usize).unwrap(),
                    data14: row.get(Data14 as usize).unwrap(),
                    data15: row.get(Data15 as usize).unwrap(),
                    data16: row.get(Data16 as usize).unwrap(),
                    data17: row.get(Data17 as usize).unwrap(),
                    data18: row.get(Data18 as usize).unwrap(),
                    data19: row.get(Data19 as usize).unwrap(),
                    data20: row.get(Data20 as usize).unwrap(),
                    data21: row.get(Data21 as usize).unwrap(),
                    data22: row.get(Data22 as usize).unwrap(),
                    data23: row.get(Data23 as usize).unwrap(),
                    min_money_loot: row.get(MinMoneyLoot as usize).unwrap(),
                    max_money_loot: row.get(MaxMoneyLoot as usize).unwrap(),
                };

                Ok(template)
            })
            .unwrap();

        result.filter_map(|res| res.ok()).collect()
    }
}

enum GameObjectTemplateColumnIndex {
    Entry,
    Type,
    DisplayId,
    Name,
    CastBarCaption,
    Faction,
    Flags,
    Size,
    Data0,
    Data1,
    Data2,
    Data3,
    Data4,
    Data5,
    Data6,
    Data7,
    Data8,
    Data9,
    Data10,
    Data11,
    Data12,
    Data13,
    Data14,
    Data15,
    Data16,
    Data17,
    Data18,
    Data19,
    Data20,
    Data21,
    Data22,
    Data23,
    MinMoneyLoot,
    MaxMoneyLoot,
}
