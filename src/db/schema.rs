table! {
    chests (id) {
        id -> Integer,
        position -> BigInt,
        lv -> SmallInt,
        found_by -> Nullable<BigInt>,
    }
}

table! {
    users (id) {
        id -> BigInt,
        uuid -> Text,
        token -> Nullable<BigInt>,
    }
}

allow_tables_to_appear_in_same_query!(chests, users);
