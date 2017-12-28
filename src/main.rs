extern crate iron;
extern crate router;

use iron::prelude;
use iron::status;
use router::Router;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::borrow::Cow;
use std::io;

fn list_posts() -> String {
    if let Ok(posts) = fs::read_dir("posts") {
        let list = posts.map(/* map to all DirEntry */
                |ent| ent.ok().map( /* FnOnce for Result of DirEntry */
                    |p| p.path().to_string_lossy().clone().to_string()
                ).unwrap());
        list.fold("".to_string(), |acc, x| { acc + x.as_ref() })

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

fn new(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    let mut writer: Vec<u8> = vec![];
    io::copy(&mut req.body, &mut writer);
    unreachable!();
}

fn show(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    unreachable!();
}

fn main() {
    let mut chain = Router::new();
    let rootdir = "/";
    chain.post("/posts/new", new, "newpost");
    chain.get("/posts/raw/:location", raw, "getpost");
    chain.get("/posts/:location", show, "showpost");

    iron::Iron::new(chain).http("localhost:3000").unwrap();
}
