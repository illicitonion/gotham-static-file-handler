extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use gotham::router::builder::{DefineSingleRoute, DrawRoutes};
use std::io::Read;
use std::path::PathBuf;

pub fn main() {
    let path = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| panic!("Need to pass an arg which is path to serve")));
    let handler = StaticFileHandler::new(path);
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);

    let router = gotham::router::builder::build_simple_router(|route| {
        route
            .get("/*")
            .with_path_extractor::<FilePath>()
            .to_new_handler(move || Ok(|state| handler.static_page::<FilePath>(state)));
    });

    gotham::start(addr, router)
}

struct StaticFileHandler {
    root: PathBuf,
}

impl StaticFileHandler {
    pub fn new(root: PathBuf) -> StaticFileHandler {
        StaticFileHandler {
            root
        }
    }

    pub fn static_page<G: GetGlob>(&self, state: gotham::state::State) -> (gotham::state::State, hyper::Response) {
        let path = {
            let glob = G::borrow_from(&state).glob();
            let mut path = self.root.clone();
            for component in glob {
                path.push(component);
            }
            path
        };
        let response = path.metadata().and_then(|meta| {
            let mut contents = Vec::with_capacity(meta.len() as usize);
            std::fs::File::open(path).and_then(|mut f| f.read_to_end(&mut contents))?;
            Ok(contents)
        }).map(|contents| gotham::http::response::create_response(
                &state,
                hyper::StatusCode::Ok,
                Some((contents, mime::TEXT_PLAIN))
            )
        ).unwrap_or_else(|err| error_response(&state, err));
        (state, response)
    }
}

fn error_response(state: &gotham::state::State, e: std::io::Error) -> hyper::Response {
    let status = match e.kind() {
        std::io::ErrorKind::NotFound => hyper::StatusCode::NotFound,
        std::io::ErrorKind::PermissionDenied => hyper::StatusCode::Forbidden,
        _ => hyper::StatusCode::InternalServerError,
    };
    gotham::http::response::create_response(
        &state,
        status,
        Some((format!("{}", status).into_bytes(), mime::TEXT_PLAIN)),
    )
}

trait GetGlob: gotham::state::FromState {
    fn glob(&self) -> &[String];
}

#[derive(Debug, Deserialize, StateData, StaticResponseExtender)]
struct FilePath {
    #[serde(rename = "*")]
    parts: Vec<String>,
}

impl GetGlob for FilePath {
    fn glob(&self) -> &[String] {
        &self.parts
    }
}
