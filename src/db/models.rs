use super::schema::chests;

#[derive(Queryable)]
pub struct Chest {
    pub id: i32,
    pub position: i64,
    pub lv: i16,
    pub found_by: String,
}

#[derive(Insertable)]
#[table_name = "chests"]
pub struct NewChest<'a> {
    pub position: i64,
    pub lv: i16,
    pub found_by: &'a str,
}
