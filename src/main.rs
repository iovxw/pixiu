#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
#![feature(non_modrs_mods)]
#![feature(crate_in_paths)]
#![feature(nll)]

extern crate base62;
#[macro_use]
extern crate lazy_static;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rand;
extern crate reqwest;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate rusqlite;
#[macro_use]
extern crate serde_derive;

mod db;
mod token;
mod minecraft;
mod errors;
#[cfg(test)]
mod tests;

use std::sync::Mutex;
use std::time::Duration;

use rocket::{State, response::status};
use rocket_contrib::{Json, Value};
use r2d2_sqlite::SqliteConnectionManager;

type Pool = r2d2::Pool<SqliteConnectionManager>;
type TokenCache = Mutex<token::UnverifiedTokenCache>;

// The URL to the database, set via the `DATABASE_URL` environment variable.
lazy_static! {
    static ref DATABASE_URL: String =
        std::env::var("DATABASE_URL").unwrap_or(concat!(env!("CARGO_PKG_NAME"), ".db").to_string());
}

/// Initializes a database pool.
fn init_pool() -> Pool {
    let manager = SqliteConnectionManager::file(DATABASE_URL.as_str());
    r2d2::Pool::new(manager).expect("db pool")
}

#[get("/put/<raw_token>/<key>/<value>")]
fn put(
    raw_token: String,
    key: String,
    value: String,
    conn: db::DbConn,
    token_cache: State<TokenCache>,
) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    let (id, token) = parse_token(&raw_token).ok_or_else(errors::forbidden)?;
    verify_token(&conn, &token_cache, id, token, &raw_token)?;

    db::put(&conn, &key, &value, id).map_err(|e| {
        println!("database error: {}", e);
        errors::database_error()
    })?;
    Ok(Json(json!({ "status": "ok" })))
}

#[get("/get/<raw_token>/<key>")]
fn get(
    raw_token: String,
    key: String,
    conn: db::DbConn,
    token_cache: State<TokenCache>,
) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    let (id, token) = parse_token(&raw_token).ok_or_else(errors::forbidden)?;
    verify_token(&conn, &token_cache, id, token, &raw_token)?;

    let data = db::get(&conn, &key).map_err(|e| {
        println!("database error: {}", e);
        errors::database_error()
    })?;
    Ok(Json(json!({ "status": "ok", "values": data })))
}

#[get("/getall/<raw_token>/<key>")]
fn getall(
    raw_token: String,
    key: String,
    conn: db::DbConn,
    token_cache: State<TokenCache>,
) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    let (id, token) = parse_token(&raw_token).ok_or_else(errors::forbidden)?;
    verify_token(&conn, &token_cache, id, token, &raw_token)?;

    let data = db::getall(&conn, &key).map_err(|e| {
        println!("database error: {}", e);
        errors::database_error()
    })?;
    Ok(Json(json!({ "status": "ok", "values": data })))
}

fn parse_token(raw_token: &str) -> Option<(u64, u64)> {
    let mut iter = raw_token.split(':');
    let id = iter.next()?;
    let token = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    let id = base62::decode(&id).ok()?;
    let token = base62::decode(&token).ok()?;
    Some((id, token))
}

fn verify_token(
    conn: &rusqlite::Connection,
    token_cache: &TokenCache,
    id: u64,
    token: u64,
    raw_token: &str,
) -> Result<(), status::Custom<Json<Value>>> {
    if db::verify_token(&conn, id, token).map_err(|e| {
        println!("database error: {}", e);
        errors::database_error()
    })? {
        Ok(())
    } else {
        if let Some(username) = token_cache.lock().unwrap().verify(id, token) {
            if minecraft::has_joined(&username, raw_token)
                .map_err(|_| errors::mojang_service_error())?
            {
                db::update_token(conn, id, token).map_err(|e| {
                    println!("database error: {}", e);
                    errors::database_error()
                })?;
                return Ok(());
            }
        }
        Err(errors::forbidden())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NewTokenReq {
    player: String,
}

#[get("/newtoken/<uuid>/<username>")]
fn newtoken(
    uuid: String,
    username: String,
    conn: db::DbConn,
    token_cache: State<TokenCache>,
) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    let user_id = db::get_user_id(&conn, &uuid).map_err(|e| {
        println!("database error: {}", e);
        errors::database_error()
    })?;
    let user_id = if user_id.is_none() {
        // TODO: verify the uuid
        db::insert_user(&conn, &uuid).map_err(|e| {
            println!("database error: {}", e);
            errors::database_error()
        })?
    } else {
        user_id.unwrap()
    };
    let token = token_cache.lock().unwrap().generate(user_id, username);
    let token = base62::encode(user_id) + ":" + &base62::encode(token);
    Ok(Json(json!({ "status": "ok", "token": token })))
}

#[error(404)]
fn not_found() -> Json<Value> {
    Json(json!({
        "status": "error",
        "reason": "Resource was not found."
    }))
}

#[error(400)]
fn bad_request() -> Json<Value> {
    Json(json!({
        "status": "error",
        "reason": "Bad request."
    }))
}

fn rocket() -> rocket::Rocket {
    let token_cache = Mutex::new(token::UnverifiedTokenCache::new(Duration::from_secs(10)));
    rocket::ignite()
        .mount("/", routes![put, get, getall, newtoken])
        .catch(errors![not_found, bad_request])
        .manage(init_pool())
        .manage(token_cache)
}

fn main() {
    rocket().launch();
}
