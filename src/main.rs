#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
#![feature(non_modrs_mods)]
#![feature(crate_in_paths)]

extern crate base62;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate lazy_static;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rand;
extern crate reqwest;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
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
use diesel::{prelude::*, sqlite::SqliteConnection};
use r2d2_diesel::ConnectionManager;

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
type TokenCache = Mutex<token::UnverifiedTokenCache>;

// The URL to the database, set via the `DATABASE_URL` environment variable.
lazy_static! {
    static ref DATABASE_URL: String = std::env::var("DATABASE_URL")
        .unwrap_or(concat!(env!("CARGO_PKG_NAME"), ".db").to_string());
}

/// Initializes a database pool.
fn init_pool() -> Pool {
    let manager = ConnectionManager::<SqliteConnection>::new(DATABASE_URL.as_str());
    r2d2::Pool::new(manager).expect("db pool")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chest {
    x: i64,
    y: i64,
    z: i64,
    lv: u8,
}

impl From<(i64, i16)> for Chest {
    fn from(src: (i64, i16)) -> Chest {
        let Position { x, y, z } = Position::from_i64(src.0);
        Chest {
            x: x,
            y: y,
            z: z,
            lv: src.1 as u8,
        }
    }
}

impl Chest {
    fn position(&self) -> Position {
        Position {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Position {
    x: i64,
    y: i64,
    z: i64,
}

impl Position {
    fn from_i64(data: i64) -> Position {
        Position {
            x: data >> 38,
            y: (data >> 26) & 0xFFF,
            z: data << 38 >> 38,
        }
    }

    fn as_i64(&self) -> i64 {
        ((self.x & 0x3FFFFFF) << 38) | ((self.y & 0xFFF) << 26) | (self.z & 0x3FFFFFF)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NewChestReq {
    chest: Chest,
}

#[post("/newchest/<raw_token>", format = "application/json", data = "<message>")]
fn newchest(
    raw_token: String,
    message: Json<NewChestReq>,
    conn: db::DbConn,
) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    let (id, token) = parse_token(&raw_token).ok_or_else(errors::forbidden)?;
    verify_token(&conn, &token_cache, id, token, &raw_token)?;

    println!("{:?}", message); // TODO: use logger
    db::insert_chest(&conn, &message.chest, id).map_err(|_| errors::database_error())?;
    Ok(Json(json!({ "status": "ok" })))
}

#[get("/chests/<raw_token>")]
fn chests(
    raw_token: String,
    conn: db::DbConn,
    token_cache: State<TokenCache>,
) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    use db::schema::chests::dsl;
    let (id, token) = parse_token(&raw_token).ok_or_else(errors::forbidden)?;
    verify_token(&conn, &token_cache, id, token, &raw_token)?;

    let data = dsl::chests
        .select((dsl::position, dsl::lv))
        .distinct()
        .load::<(i64, i16)>(&*conn)
        .map_err(|_| errors::database_error())?;
    let data: Vec<Chest> = data.into_iter().map(Into::into).collect();
    Ok(Json(json!({ "status": "ok", "chests": data })))
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
    conn: &SqliteConnection,
    token_cache: &TokenCache,
    id: u64,
    token: u64,
    raw_token: &str,
) -> Result<(), status::Custom<Json<Value>>> {
    if db::verify_token(&conn, id, token).map_err(|_| errors::database_error())? {
        Ok(())
    } else {
        if let Some(username) = token_cache.lock().unwrap().verify(id, token) {
            if minecraft::has_joined(&username, raw_token)
                .map_err(|_| errors::mojang_service_error())?
            {
                db::update_token(conn, id, token).map_err(|_| errors::database_error())?;
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
    let user_id = db::get_user_id(&conn, &uuid).map_err(|_| errors::database_error())?;
    let user_id = if user_id.is_none() {
        // TODO: verify the uuid
        db::insert_user(&conn, &uuid).map_err(|_| errors::database_error())?
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
        .mount("/", routes![newchest, chests, newtoken])
        .catch(errors![not_found, bad_request])
        .manage(init_pool())
        .manage(token_cache)
}

fn main() {
    rocket().launch();
}
