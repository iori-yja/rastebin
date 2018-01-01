extern crate base64;
extern crate iron;
extern crate router;
extern crate rand;

use base64::encode_config;
use iron::status;
use rand::{Rng, thread_rng};
use router::Router;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::prelude;
use std::io;
use std::io::Read;

fn list_posts() -> String {
    if let Ok(posts) = fs::read_dir("posts") {
        let list = posts.map(/* map to all DirEntry */
                |ent| ent.ok().map( /* FnOnce for Result of DirEntry */
                    |p| p.path().to_string_lossy().clone().to_string()
                ).unwrap());
        list.fold("".to_string(), |acc, x| { acc + "\n" + x.as_ref() })

    } else {
        "".to_string()
    }
}

fn find_post(loc: &str) -> Option<Vec<u8>> {
    fs::File::open(loc).map(|x| x.bytes().map(|x| x.unwrap()).collect()).ok()
}

fn raw(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    let loc = req.extensions.get::<Router>().unwrap().find("location");
    if loc.is_none() {
        Ok(iron::Response::with((status::Ok, list_posts())))
    } else {
        if let Some(post) = find_post(loc.unwrap()) {
            Ok(iron::Response::with((status::Ok, post)))
        } else {
            Ok(iron::Response::with((status::NotFound, "")))
        }
    }
}

fn generate_random_name() -> String {
    let mut rng = thread_rng();
    let candidate = encode_config(& rng.gen_iter::<u8>().take(4).collect::<Vec<u8>>(), base64::URL_SAFE_NO_PAD);
    println!("{}\n", candidate);
    return candidate;
}

fn new(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    let fname = generate_random_name();
    let mut writer = File::create(fname).unwrap();
    let mut copied = io::copy(&mut req.body, &mut writer);
    match copied {
        Ok(_) => Ok(iron::Response::with((status::Ok, ""))),
        Err(e) => Ok(iron::Response::with((status::InternalServerError, e.description()))),
    }
}

fn show(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    unreachable!();
}

fn main() {
    let mut chain = Router::new();
    chain.post("/posts/new", new, "newpost");
    chain.get("/posts/raw/:location", raw, "getpost");
    chain.get("/posts/:location", show, "showpost");

    iron::Iron::new(chain).http("localhost:3000").unwrap();
}
