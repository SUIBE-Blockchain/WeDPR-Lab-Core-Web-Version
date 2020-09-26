#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use std::sync::Mutex;
use std::collections::HashMap;

use rocket::State;
use rocket_contrib::json::{Json, JsonValue};

// Copyright 2020 WeDPR Lab Project Authors. Licensed under Apache-2.0.

use colored::*;
use std;
use verifiable_confidential_ledger::vcl;
//use wedpr_crypto;
use wedpr_crypto::{
    self,
    constant::{BASEPOINT_G1, BASEPOINT_G2},
    utils::{point_to_string, scalar_to_string,string_to_scalar},
};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Secrets{
    // c1: {
        pub secret_blinding: String,
        pub credit_value: u64,
    // }
    // c2_secret: String,
    // c3_secret: String
}

// The type to represent the ID of a message.
type ID = usize;

// We're going to store all of the messages here. No need for a DB.
type MessageMap = Mutex<HashMap<ID, String>>;

#[derive(Serialize, Deserialize)]
struct Message {
    id: Option<ID>,
    contents: String
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

#[post("/prove_range", format = "json", data = "<secrets>")]
fn prove_range(secrets: Json<Secrets>) -> JsonValue {
    let secret_blinding_scalar=string_to_scalar(&secrets.secret_blinding).unwrap();
    let owner_secret= vcl::OwnerSecret {
        credit_value: secrets.credit_value,
        secret_blinding: secret_blinding_scalar,
    };
    let range_proof_c1 = vcl::prove_range(&owner_secret);
    println!("非负数的证明数据：\n{:?}", range_proof_c1);
    json!({ "status": "ok","result":  range_proof_c1})
}

// TODO: This example can be improved by using `route` with multiple HTTP verbs.
#[post("/<id>", format = "json", data = "<message>")]
fn new(id: ID, message: Json<Message>, map: State<MessageMap>) -> JsonValue {
    let mut hashmap = map.lock().expect("map lock.");
    if hashmap.contains_key(&id) {
        json!({
            "status": "error",
            "reason": "ID exists. Try put."
        })
    } else {
        hashmap.insert(id, message.0.contents);
        json!({ "status": "ok" })
    }
}

#[put("/<id>", format = "json", data = "<message>")]
fn update(id: ID, message: Json<Message>, map: State<MessageMap>) -> Option<JsonValue> {
    let mut hashmap = map.lock().unwrap();
    if hashmap.contains_key(&id) {
        hashmap.insert(id, message.0.contents);
        Some(json!({ "status": "ok" }))
    } else {
        None
    }
}

#[get("/<id>", format = "json")]
fn get(id: ID, map: State<MessageMap>) -> Option<Json<Message>> {
    let hashmap = map.lock().unwrap();
    hashmap.get(&id).map(|contents| {
        Json(Message {
            id: Some(id),
            contents: contents.clone()
        })
    })
}

#[catch(404)]
fn not_found() -> JsonValue {
    json!({
        "status": "error",
        "reason": "Resource was not found."
    })
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/vcl",routes![prove_range])
        .mount("/message", routes![new, update, get])
        .register(catchers![not_found])
        .manage(Mutex::new(HashMap::<ID, String>::new()))
}

fn main() {
    rocket().launch();
}
