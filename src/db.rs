use std::ops::Deref;

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
    found_by: &str,
) -> Result<(), diesel::result::Error> {
    use self::schema::chests;

    let new_chest = models::NewChest {
        position: chest.position().as_i64(),
        lv: chest.lv as i16,
        found_by: found_by,
    };

    diesel::insert_into(chests::table)
        .values(&new_chest)
        .execute(conn)?;
    Ok(())
}
