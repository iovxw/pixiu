use rocket::{http::Status, response::status};
use rocket_contrib::{Json, Value};

#[inline]
pub fn forbidden() -> status::Custom<Json<Value>> {
    status::Custom(
        Status::Forbidden,
        Json(json!({
            "status": "error",
            "reason": "Forbidden."
        })),
    )
}

#[inline]
pub fn database_error() -> status::Custom<Json<Value>> {
    status::Custom(
        Status::InternalServerError,
        Json(json!({
            "status": "error",
            "reason": "Database error."
        })),
    )
}

#[inline]
pub fn mojang_service_error() -> status::Custom<Json<Value>> {
    status::Custom(
        Status::InternalServerError,
        Json(json!({
            "status": "error",
            "reason": "Minecraft Session Server is down, please try again later."
        })),
    )
}
