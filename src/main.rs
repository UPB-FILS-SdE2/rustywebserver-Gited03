use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use std::convert::Infallible;
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio::process::Command;

static NOTFOUND: &[u8] = b"Not Found";

async fn handle_request(req: Request<Body>, root_folder: PathBuf) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method();

    let mut response = Response::new(Body::empty());

    if method == Method::GET {
        let file_path = root_folder.join(&path[1..]);
        if file_path.exists() && file_path.is_file() {
            match fs::read(&file_path) {
                Ok(contents) => {
                    let mime_type = match file_path.extension().and_then(|ext| ext.to_str()) {
                        Some("html") => "text/html; charset=utf-8",
                        Some("css") => "text/css; charset=utf-8",
                        Some("js") => "text/javascript; charset=utf-8",
                        Some("jpg") | Some("jpeg") => "image/jpeg",
                        Some("png") => "image/png",
                        Some("zip") => "application/zip",
                        _ => "application/octet-stream",
                    };
                    response.headers_mut().insert(hyper::header::CONTENT_TYPE, mime_type.parse().unwrap());
                    *response.body_mut() = Body::from(contents);
                },
                Err(_) => {
                    *response.status_mut() = StatusCode::FORBIDDEN;
                },
            }
        } else {
            *response.status_mut() = StatusCode::NOT_FOUND;
            *response.body_mut() = Body::from(NOTFOUND);
        }
    } else if method == Method::POST {
        if path.starts_with("/scripts/") {
            let script_path = root_folder.join(&path[1..]);
            if script_path.exists() && script_path.is_file() {
                match Command::new(script_path).output().await {
                    Ok(output) => {
                        *response.status_mut() = StatusCode::OK;
                        *response.body_mut() = Body::from(output.stdout);
                    },
                    Err(_) => {
                        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    },
                }
            } else {
                *response.status_mut() = StatusCode::NOT_FOUND;
                *response.body_mut() = Body::from(NOTFOUND);
            }
        } else {
            *response.status_mut() = StatusCode::NOT_FOUND;
            *response.body_mut() = Body::from(NOTFOUND);
        }
    } else {
        *response.status_mut() = StatusCode::NOT_FOUND;
        *response.body_mut() = Body::from(NOTFOUND);
    }

    Ok(response)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} PORT ROOT_FOLDER", args[0]);
        return;
    }

    let port: u16 = args[1].parse().unwrap();
    let root_folder = PathBuf::from(&args[2]);

    let addr = ([0, 0, 0, 0], port).into();

    println!("Root folder: {:?}", root_folder);
    println!("Server listening on 0.0.0.0:{}", port);

    let make_svc = make_service_fn(move |_conn| {
        let root_folder = root_folder.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, root_folder.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        println!("server error: {}", e);
    }
}
