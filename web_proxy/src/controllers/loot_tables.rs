use actix_web::{get, http::header::ContentType, post, put, web, HttpResponse, Responder};
use shared::{
    models::loot::{CreateLootTable, LootTable, UpdateLootTable},
    repositories::loot::LootRepository,
};

use crate::{services, wowhead, WorldDb, WowheadCacheDb};

#[post("/{entity_type}/{entry}/lootTable")]
pub async fn create_loot_table(
    db_pool: web::Data<WorldDb>,
    path: web::Path<(String, u32)>,
    loot_table: web::Json<CreateLootTable>,
) -> actix_web::Result<impl Responder> {
    let (entity_type, entity_id) = path.into_inner();

    let loot_table: LootTable = web::block(move || {
        let mut conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");
        let next_loot_table_id =
            services::loot::get_next_loot_table_id(&conn, &entity_type, entity_id);

        let db_payload = UpdateLootTable {
            id: next_loot_table_id,
            groups: loot_table.0.groups,
        };

        LootRepository::replace_loot_table(&mut conn, entity_id, db_payload);
        LootRepository::fetch_loot_table_by_id(&conn, next_loot_table_id).unwrap()
    })
    .await?
    .expect("cannot find the loot table that we just created");

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&loot_table).unwrap()))
}

#[put("/spawn/{entry}/lootTable")]
pub async fn update_loot_table(
    db_pool: web::Data<WorldDb>,
    path: web::Path<u32>,
    loot_table: web::Json<UpdateLootTable>,
) -> actix_web::Result<impl Responder> {
    let template_id = path.into_inner();
    let updated_loot_table: LootTable = web::block(move || {
        let mut conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");
        let loot_table_id = loot_table.0.id;

        LootRepository::replace_loot_table(&mut conn, template_id, loot_table.0);
        LootRepository::fetch_loot_table_by_id(&conn, loot_table_id).unwrap()
    })
    .await?
    .expect("cannot find the loot table that we just updated");

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&updated_loot_table).unwrap()))
}

#[get("/{entity_type}/{id}/referenceLootTable")]
pub async fn fetch_loot_table_from_wowhead(
    db_pool: web::Data<WowheadCacheDb>,
    path: web::Path<(String, u32)>,
) -> actix_web::Result<impl Responder> {
    let (entity_type, id) = path.into_inner();

    let maybe_loot_table = web::block(move || {
        let conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");
        wowhead::service::get_loot_table(&conn, entity_type.try_into().unwrap(), id)
    })
    .await?;

    match maybe_loot_table {
        Some(loot_table) => Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(serde_json::to_string(&loot_table).unwrap())),
        None => Ok(HttpResponse::NotFound().into()),
    }
}
