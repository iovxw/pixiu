use super::schema::*;

#[derive(Insertable)]
#[table_name = "chests"]
pub struct NewChest {
    pub position: i64,
    pub lv: i16,
    pub found_by: i64,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub uuid: &'a str,
}
