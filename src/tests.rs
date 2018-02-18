use rocket;
use rocket::local::Client;
use rocket::http::{ContentType, Status};

#[test]
fn post_newchest() {
    let client = Client::new(rocket()).unwrap();

    let res = client
        .post("/newchest")
        .header(ContentType::JSON)
        .body(
            r#"{
                     "player": {
                         "name": "bob",
                         "uuid": "blahblah"
                     },
                     "chest": {
                         "x":1,
                         "y":2,
                         "z": 3,
                         "lv": 4
                     }
                 }"#,
        )
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
}

#[test]
fn position_encode() {
    use super::*;
    let data = Position {
        x: -846,
        y: 90,
        z: -964,
    };
    assert_eq!(data.as_i64(), -232540602368964);
}

#[test]
fn position_decode() {
    use super::*;
    let data = Position {
        x: -846,
        y: 90,
        z: -964,
    };
    assert_eq!(Position::from_i64(-232540602368964), data);
}
