use actix_web::{get, http::header::ContentType, web, HttpResponse, Responder};

use crate::{
    repositories::spawns::SpawnsRepository, Bounds, CreatureSpawnDbRecord, CreatureTemplate,
    WorldDb,
};

#[get("/spawns")]
pub async fn get_spawns(
    db_pool: web::Data<WorldDb>,
    bounds: web::Query<Bounds>,
) -> actix_web::Result<impl Responder> {
    let spawns: Vec<CreatureSpawnDbRecord> = web::block(move || {
        let bounds = bounds.0;

        // Obtaining a connection from the pool is also a potentially blocking operation.
        // So, it should be called within the `web::block` closure, as well.
        let conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");

        SpawnsRepository::get_spawns_in_bounds(&conn, &bounds)
    })
    .await?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&spawns).unwrap()))
}

#[get("/spawn/{entry}")]
async fn get_template(
    db_pool: web::Data<WorldDb>,
    path: web::Path<u32>,
) -> actix_web::Result<impl Responder> {
    let template: Option<CreatureTemplate> = web::block(move || {
        let conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");

        SpawnsRepository::get_creature_template(&conn, path.into_inner())
    })
    .await?;

    if let Some(template) = template {
        Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(serde_json::to_string(&template).unwrap()))
    } else {
        Ok(HttpResponse::NotFound().into())
    }
}
