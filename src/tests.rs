use rocket;
use rocket::local::Client;
use rocket::http::{Status, ContentType};

#[test]
fn post_newchest() {
    let client = Client::new(rocket()).unwrap();

    let res = client.post("/newchest")
        .header(ContentType::JSON)
        .body(r#"{
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
                 }"#)
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
}
