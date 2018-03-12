use std::ops::Deref;
use std::mem::transmute;

use rusqlite;
use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;

use super::Pool;
use super::Chest;

// Connection request guard type: a wrapper around an r2d2 pooled connection.
pub struct DbConn(pub r2d2::PooledConnection<SqliteConnectionManager>);

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
    type Target = rusqlite::Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn insert_chest(
    conn: &rusqlite::Connection,
    chest: &Chest,
    found_by: u64,
) -> Result<(), rusqlite::Error> {
    const SQL: &str = "INSERT INTO chests (position, lv, found_by) VALUES (?1, ?2, ?3)";
    let mut insert = conn.prepare_cached(SQL).expect(SQL);
    insert.insert(&[&chest.position().as_i64(), &chest.lv, &u64_to_i64(found_by)])?;
    Ok(())
}

pub fn all_chests(conn: &rusqlite::Connection) -> Result<Vec<super::Chest>, rusqlite::Error> {
    use super::{Position, Chest};

    const SQL: &str = "SELECT DISTINCT position, lv FROM chests";
    let mut get_id = conn.prepare_cached(SQL).expect(SQL);
    let mut rows = get_id.query(&[])?;

    let mut r = Vec::new();

    while let Some(result_row) = rows.next() {
        let row = result_row?;
        let Position { x, y, z } = Position::from_i64(row.get(0));
        let lv = row.get::<i32,i64>(1) as u8;
        let chest = Chest {x,y,z,lv};
        r.push(chest);
    }

    Ok(r)
}

pub fn insert_user(conn: &rusqlite::Connection, user: &str) -> Result<u64, rusqlite::Error> {
    const SQL: &str = "INSERT INTO users (uuid) VALUES (?)";
    let mut insert = conn.prepare_cached(SQL).expect(SQL);
    let id = insert.insert(&[&user])?;
    Ok(id as u64)
}

pub fn get_user_id(
    conn: &rusqlite::Connection,
    user: &str,
) -> Result<Option<u64>, rusqlite::Error> {
    const SQL: &str = "SELECT id FROM users WHERE uuid = ? LIMIT 1";
    let mut get_id = conn.prepare_cached(SQL).expect(SQL);
    let mut rows = get_id.query(&[&user])?;

    if let Some(row) = rows.next() {
        Ok(Some(row?.get::<i32, i64>(0) as u64))
    } else {
        Ok(None)
    }
}

pub fn update_token(
    conn: &rusqlite::Connection,
    user: u64,
    token: u64,
) -> Result<bool, rusqlite::Error> {
    const SQL: &str = "UPDATE users SET token = ?1 WHERE id = ?2";
    let mut update = conn.prepare_cached(SQL).expect(SQL);
    let n = update.execute(&[&u64_to_i64(token), &(user as i64)])?;

    Ok(n != 0)
}

pub fn verify_token(
    conn: &rusqlite::Connection,
    user: u64,
    token: u64,
) -> Result<bool, rusqlite::Error> {
    const SQL: &str = "SELECT id FROM users WHERE id = ?1 AND token = ?2 LIMIT 1";
    let mut query = conn.prepare_cached(SQL).expect(SQL);
    let mut rows = query.query(&[&(user as i64), &u64_to_i64(token)])?;

    if let Some(row) = rows.next() {
        row?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn u64_to_i64(u: u64) -> i64 {
    unsafe { transmute::<u64, i64>(u) }
}
