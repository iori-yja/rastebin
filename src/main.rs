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
use std::{io, fs, mem};
use std::fs::File;
use std::prelude;
use std::io::{Read, Write, BufReader, BufWriter, Cursor, SeekFrom, Seek};

type PostMeta = (Option<String>, Option<String>, Option<String>);

fn describe_post(fname: &str) -> PostMeta {
    let mut desc = String::new();
    if let Ok(mut file) = fs::File::open(format!("metadata/{}", fname)).map(|x| BufReader::new(x)) {
        file.read_to_string(&mut desc);
        let mut csv = desc.split(',').map(|x| x.into());
        (csv.next(), csv.next(), csv.next())
    } else {
        return (None, None, None)
    }
}

fn list_posts() -> Option<Vec<(String, PostMeta)>> {
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
    let mut fname = generate_random_name();
    let mut open = File::open(format!("posts/{}", &fname));
    while open.is_ok() {
        fname = fname + "_";
        open = File::open(format!("posts/{}", &fname));
    }
    let mut writer = BufWriter::new(File::create(format!("posts/{}", &fname)).unwrap());

    let mut buf: Vec<u8> = Vec::new();
    let mut request_buffer = Cursor::new(buf);

    let time = Local::now();

    let mut copied = 0;
    loop {
        match std::io::copy(&mut req.body, &mut request_buffer) {
            Ok(mut c) => {
                if c == 0 {break};
                if copied == 0 {
                    /* our http request contains 8bytes string as a header;
                     * which is "content=" */
                    request_buffer.seek(SeekFrom::Start(8));
                    /* substitute the offset */
                    c -= 8;
                }
                std::io::copy(&mut request_buffer, &mut writer);
                copied += c;
            },
            Err(e) => {
                println!("{}", e);
                break;
            }
        }
    }

    println!("Created {} ({}bytes) at {} by request from {}", fname, copied, time, req.remote_addr);
    let mut meta = BufWriter::new(File::create(format!("metadata/{}", fname)).unwrap());
    /* The format of metadata is CSV; specifically, see below */
    meta.write(format!("{},{},{}", copied, time, req.remote_addr).as_bytes()).unwrap();
    Ok(iron::Response::with((status::Ok, fname)))
}

fn form(_: &mut iron::Request) -> iron::IronResult<iron::Response> {
    Ok(iron::Response::with((status::Ok, iron::headers::ContentType::html().0, include_str!("form.html"))))
}

fn showtable(_: &mut iron::Request) -> iron::IronResult<iron::Response> {
        let resp_before = "<html><body><table><tr><th></th><th>size</th><th>timestamp</th><th>origin</th></tr>";
    let posts = list_posts()
                .map(|x| x.iter()
                     .fold(String::new(),
                        |acc, p| acc + format!("<tr><td><a href={p}><tt>{p}</tt></a></td><td align=\"right\">{d0}bytes</td><td align=\"center\">{d1}</td><td align=\"center\">{d2}</td></tr>",
                                               p=p.0,
                                               d0=(p.1.clone()).0.unwrap_or("?".into()),
                                               d1=(p.1.clone()).1.unwrap_or("?".into()),
                                               d2=(p.1.clone()).2.unwrap_or("unknown".into())).as_ref()));

    let resp_after = "</table></body></html>";
    Ok(iron::Response::with((iron::headers::ContentType::html().0, status::Ok, resp_before.to_string() + posts.unwrap_or("".to_string()).as_ref() + resp_after)))
}

fn show(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    let loc = req.extensions.get::<Router>().unwrap().find("location");
    if let Ok(post) = find_post(&format!("posts/{}", loc.unwrap())) {
        let mut body: String = "".to_string();
        post.take(2048).read_to_string(&mut body);
        let res = format!(include_str!("template.html"),
                          title=loc.unwrap(),
                          body=htmlescape::encode_minimal(&body),
                          body_header="",
                          body_footer="");
        Ok(iron::Response::with((iron::headers::ContentType::html().0, status::Ok, res)))
    } else {
        Ok(iron::Response::with((status::NotFound, "")))
    }
}

fn main() {
    let mut chain = Router::new();
    chain.post("/posts/new", new, "newpost");
    chain.get("/posts/new", form, "newform");
    chain.get("/posts/raw/:location", raw, "getpost");
    chain.get("/posts/", showtable, "listpost");
    chain.get("/posts/:location", show, "showpost");

    iron::Iron::new(chain).http("localhost:3000").unwrap();
}
