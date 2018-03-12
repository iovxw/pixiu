use std::ops::Deref;
use std::mem::transmute;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use r2d2;
use r2d2_diesel::ConnectionManager;

use super::Pool;
use super::Chest;

pub mod models;
pub mod schema;

// Connection request guard type: a wrapper around an r2d2 pooled connection.
pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<SqliteConnection>>);

/// Attempts to retrieve a single connection from the managed database pool. If
/// no pool is currently managed, fails with an `InternalServerError` status. If
/// no connections are available, fails with a `ServiceUnavailable` status.
impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<DbConn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

// For the convenience of using an &DbConn as an &SqliteConnection.
impl Deref for DbConn {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn insert_chest(
    conn: &SqliteConnection,
    chest: &Chest,
    found_by: u64,
) -> Result<(), diesel::result::Error> {
    use self::schema::chests;

    let new_chest = models::NewChest {
        position: chest.position().as_i64(),
        lv: chest.lv as i16,
        found_by: u64_to_i64(found_by),
    };

    diesel::insert_into(chests::table)
        .values(&new_chest)
        .execute(conn)?;
    Ok(())
}

pub fn insert_user(conn: &SqliteConnection, user: &str) -> Result<u64, diesel::result::Error> {
    use self::schema::users;

    diesel::insert_into(users::table)
        .values(&models::NewUser { uuid: user })
        .execute(conn)?;
    let r = users::dsl::users
        .filter(users::dsl::uuid.eq(user))
        .select(users::dsl::id)
        .first::<i64>(conn)?;
    Ok(r as u64)
}

pub fn get_user_id(
    conn: &SqliteConnection,
    user: &str,
) -> Result<Option<u64>, diesel::result::Error> {
    use self::schema::users::dsl;
    use diesel::result::Error;

    match dsl::users
        .filter(dsl::uuid.eq(user))
        .select(dsl::id)
        .first::<i64>(conn)
    {
        Ok(id) => Ok(Some(i64_to_u64(id))),
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn update_token(
    conn: &SqliteConnection,
    user: u64,
    token: u64,
) -> Result<bool, diesel::result::Error> {
    use self::schema::users::dsl;
    use diesel::{update, result::Error};

    match update(dsl::users.find(u64_to_i64(user)))
        .set(dsl::token.eq(u64_to_i64(token)))
        .execute(conn)
    {
        Ok(_) => Ok(true),
        Err(Error::NotFound) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn verify_token(
    conn: &SqliteConnection,
    user: u64,
    token: u64,
) -> Result<bool, diesel::result::Error> {
    use self::schema::users::dsl;
    use diesel::{select, dsl::exists};
    let r = select(exists(
        dsl::users
            .filter(dsl::id.eq(u64_to_i64(user)))
            .filter(dsl::token.eq(u64_to_i64(token))),
    )).get_result::<bool>(conn)?;
    Ok(r)
}

fn i64_to_u64(i: i64) -> u64 {
    unsafe { transmute::<i64, u64>(i) }
}

fn u64_to_i64(u: u64) -> i64 {
    unsafe { transmute::<u64, i64>(u) }
}
