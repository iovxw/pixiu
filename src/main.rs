#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
#![feature(non_modrs_mods)]
#![feature(crate_in_paths)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate lazy_static;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

mod db;
#[cfg(test)]
mod tests;

use rocket::{http::Status, response::status};
use rocket_contrib::{Json, Value};
use diesel::{prelude::*, sqlite::SqliteConnection};
use r2d2_diesel::ConnectionManager;

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

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
struct Player {
    name: String,
    uuid: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewChestReq {
    player: Player,
    chest: Chest,
}

#[post("/", format = "application/json", data = "<message>")]
fn newchest(
    message: Json<NewChestReq>,
    conn: db::DbConn,
) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    println!("{:?}", message); // TODO: use logger
    if db::insert_chest(&conn, &message.chest, &message.player.uuid).is_err() {
        return Err(status::Custom(
            Status::InternalServerError,
            Json(json!({
                "status": "error",
                "reason": "Database error."
            })),
        ));
    };
    Ok(Json(json!({ "status": "ok" })))
}

#[get("/")]
fn chests(conn: db::DbConn) -> Result<Json<Value>, status::Custom<Json<Value>>> {
    use db::schema::chests::dsl::*;
    let data: Vec<Chest> = if let Ok(data) = chests
        .select((position, lv))
        .distinct()
        .load::<(i64, i16)>(&*conn)
    {
        data.into_iter().map(Into::into).collect()
    } else {
        return Err(status::Custom(
            Status::InternalServerError,
            Json(json!({
                "status": "error",
                "reason": "Database error."
            })),
        ));
    };
    Ok(Json(json!({ "status": "ok", "data": data })))
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
    rocket::ignite()
        .mount("/newchest", routes![newchest])
        .mount("/chests", routes![chests])
        .catch(errors![not_found, bad_request])
        .manage(init_pool())
}

fn main() {
    rocket().launch();
}
