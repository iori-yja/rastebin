extern crate base64;
extern crate chrono;
extern crate iron;
extern crate router;
extern crate rand;
extern crate htmlescape;

use base64::encode_config;
use chrono::offset::Local;
use iron::status;
use iron::response;
use rand::{Rng, thread_rng};
use router::Router;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::prelude;
use std::io;
use std::io::{Read, Write, BufReader, BufWriter};

fn describe_post(fname: &str) -> String {
    let mut desc = String::new();
    if let Ok(mut file) = fs::File::open(format!("metadata/{}.metadata", fname)).map(|x| BufReader::new(x)) {
        file.read_to_string(&mut desc);
        return desc;
    } else {
        return "unknown".into();
    }
}

fn list_posts() -> Option<Vec<(String, String)>> {
    if let Ok(posts) = fs::read_dir("posts") {
        let list = posts.map(/* map to all DirEntry */
                |ent| ent.ok().map( /* FnOnce for Result of DirEntry */
                    |p| p.path().file_name().unwrap().to_string_lossy().to_string()
                ).unwrap());
        Some(list.map(|l| (l.clone(), describe_post(&l))).collect())
    } else {
        None
    }
}

fn find_post(loc: &str) -> std::io::Result<BufReader<File>> {
    fs::File::open(loc).map(|x| BufReader::new(x))
}

fn raw(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    let loc = req.extensions.get::<Router>().unwrap().find("location");
    if loc.is_none() {
        unreachable!();
    } else {
        if let Ok(post) = find_post(&format!("posts/{}", loc.unwrap())) {
            Ok(iron::Response::with((status::Ok, iron::headers::ContentType::plaintext().0, response::BodyReader(post))))
        } else {
            Ok(iron::Response::with((status::NotFound, "")))
        }
    }
}

fn generate_random_name() -> String {
    let mut rng = thread_rng();
    let candidate = encode_config(& rng.gen_iter::<u8>().take(6).collect::<Vec<u8>>(), base64::URL_SAFE_NO_PAD);
    return candidate;
}

fn new(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    let mut fname = format!("posts/{}", generate_random_name());
    let mut open = File::open(&fname);
    while open.is_ok() {
        fname = fname + "_";
        open = File::open(&fname);
    }
    let writer = File::create(&fname);

    let copied = io::copy(&mut req.body, &mut writer.unwrap());
    match copied {
        Ok(byte) => {
            let metalog = format!("Created {} ({}bytes) at {} by request from {}", fname, byte, Local::now(), req.remote_addr);
            let mut meta = BufWriter::new(File::create(fname.clone() + ".metadata").unwrap());
            println!("{}", metalog);
            meta.write(metalog.as_bytes()).unwrap();
            Ok(iron::Response::with((status::Ok, fname)))
        },
        Err(e) => Ok(iron::Response::with((status::InternalServerError, e.description()))),
    }
}

fn form(_: &mut iron::Request) -> iron::iron::IronResult<Iron::Response> {
    Ok(iron::Response::with((status::Ok, iron::headers::ContentType::html().0, include_str!("template.html"))))
}

fn show(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    let loc = req.extensions.get::<Router>().unwrap().find("location");
    if loc.is_none() {
        let resp_before = "<html><body><table><tr><th></th><th>description</th></tr>";
        let posts = list_posts()
                    .map(|x| x.iter()
                         .fold(String::new(),
                            |acc, p| acc + format!("<tr><td><a href={p}><tt>{p}</tt></a></td><td>{d}</td></tr>", p=p.0, d=p.1).as_ref()));
        let resp_after = "</table></body></html>";
        Ok(iron::Response::with((iron::headers::ContentType::html().0, status::Ok, resp_before.to_string() + posts.unwrap().as_ref() + resp_after)))
    } else {
        if let Ok(post) = find_post(&format!("posts/{}", loc.unwrap())) {
            let mut body: String = "".to_string();
            post.take(2048).read_to_string(&mut body);
            let res = format!(include_str!("template.html"),
                              title=loc.unwrap(),
                              body=htmlescape::encode_minimal(&body));
            Ok(iron::Response::with((iron::headers::ContentType::html().0, status::Ok, res)))
        } else {
            Ok(iron::Response::with((status::NotFound, "")))
        }
    }
}

fn main() {
    let mut chain = Router::new();
    chain.post("/posts/new", new, "newpost");
    chain.get("/posts/new", form, "newform");
    chain.get("/posts/raw/:location", raw, "getpost");
    chain.get("/posts/:location", show, "showpost");
    chain.get("/posts/", show, "listpost");

    iron::Iron::new(chain).http("localhost:3000").unwrap();
}
