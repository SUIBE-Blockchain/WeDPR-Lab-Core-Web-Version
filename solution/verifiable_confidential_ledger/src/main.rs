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
    utils::{point_to_string, string_to_point, scalar_to_string,string_to_scalar},
};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Secret{
    pub secret_blinding: String,
    pub credit_value: u64
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Secrets{
    c1: Secret,
    c2: Secret,
    c3: Secret,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Value{
    pub value: u64,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct VerifyRangeData{
    pub credit: String,
    pub proof: String
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct VerifySumBalanceData{
    pub credit: String,
    pub proof: String
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

#[post("/make_credit", format = "json", data = "<payload>")]
fn make_credit(payload: Json<Value>) -> JsonValue {
    let (value_credit, value_secret) = vcl::make_credit(payload.value);
    let value_credit_str = wedpr_crypto::utils::point_to_string(&value_credit.get_point());
    let secret_blinding_str=scalar_to_string(&value_secret.secret_blinding);
    json!({ "status": "ok", "result":  {"credit": value_credit_str,
    "owner_secret":{"credit_value": value_secret.credit_value, "secret_blinding": secret_blinding_str}}})
}

fn str_to_secret(secret: &Secret) -> vcl::OwnerSecret {
    let secret_blinding_scalar
        =string_to_scalar(&secret.secret_blinding).unwrap();
    vcl::OwnerSecret {
        credit_value: secret.credit_value,
        secret_blinding: secret_blinding_scalar,
    }

}

#[post("/prove_sum_balance", format = "json", data = "<secrets>")]
fn prove_sum_balance(secrets: Json<Secrets>) -> JsonValue {
    let c1_secret = str_to_secret(&secrets.c1);
    let c2_secret = str_to_secret(&secrets.c2);
    let c3_secret = str_to_secret(&secrets.c3);
    let sum_proof = vcl::prove_sum_balance(&c1_secret, &c2_secret, &c3_secret);
    json!({"status": "ok", "result": 
        {
            "c": sum_proof.c,
            "m1": sum_proof.m1,
            "m2": sum_proof.m2,
            "m3": sum_proof.m3,
            "m4": sum_proof.m4,
            "m5": sum_proof.m5
        }
    })
}

#[post("/prove_range", format = "json", data = "<secrets>")]
fn prove_range(secrets: Json<Secrets>) -> JsonValue {
    let owner_secret = str_to_secret(&secrets.c1);
    let range_proof_c1 = vcl::prove_range(&owner_secret);
    println!("非负数的证明数据：\n{:?}", range_proof_c1);
    json!({ "status": "ok","result":  range_proof_c1})
}

#[post("verify_sum_balance",format = "json", data="<verify_sum_balance_data>")]
fn verify_sum_balance(verify_sum_balance_data: Json<VerifySumBalanceData>) -> JsonValue {
    
}

#[post("/verify_range", format= "json", data = "<verify_range_data>")]
fn verify_range(verify_range_data: Json<VerifyRangeData>) -> JsonValue {
    let confidential_credit = vcl::ConfidentialCredit{
        point: string_to_point(&verify_range_data.credit).unwrap()
    };
    let meet_range_constraint = vcl::verify_range(&confidential_credit, &verify_range_data.proof);
    json!({ "status": "ok", "result": meet_range_constraint})
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
        .mount("/vcl",routes![prove_range,make_credit,verify_range,prove_sum_balance])
        .mount("/message", routes![new, update, get])
        .register(catchers![not_found])
        .manage(Mutex::new(HashMap::<ID, String>::new()))
}

fn main() {
    rocket().launch();
}
