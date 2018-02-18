table! {
    chests (id) {
        id -> Integer,
        position -> BigInt,
        lv -> SmallInt,
        found_by -> Nullable<Text>,
    }
}
