/*
cd /tank/iso/rust/satisfactory_serve_map/
nix develop
cargo run -- --save-dir /tank/containers/satisfactory/saved/server
https://satisfactory-calculator.com/en/interactive-map?url=https://sf2.kemichal.com/map/Noobville
 */

use std::path::Path;

use anyhow::Result;
use clap::Parser;
use glob::glob;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::NamedFile;
use rocket::http::Header;
use rocket::response::Response;
use rocket::Request;
use rocket::State;
mod rocket_anyhow;

#[macro_use]
extern crate rocket;

// Command line arguments structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory containing save files
    #[arg(short, long, default_value = "saves")]
    save_dir: String,

    /// Port to run the server on
    #[arg(short, long, default_value = "7778")]
    port: u16,
}

// Custom error type for more informative responses
#[derive(Debug, Responder)]
enum MapError {
    #[response(status = 404)]
    NotFound(String),
    #[response(status = 400)]
    BadRequest(String),
}

// State structure to hold our configuration
struct ServerConfig {
    save_dir: String,
}

#[get("/map/<name>")]
async fn serve_map(name: &str, config: &State<ServerConfig>) -> Result<NamedFile, MapError> {
    // Basic input validation
    if name.contains(['/', '\\', '.']) {
        return Err(MapError::BadRequest("Invalid characters in name".into()));
    }

    let pattern = format!("{}/{}*.sav", config.save_dir, name);

    let latest_save = glob(&pattern)
        .map_err(|e| MapError::BadRequest(format!("Invalid pattern: {}", e)))?
        .filter_map(Result::ok)
        .filter_map(|path| {
            path.metadata()
                .ok()
                .map(|metadata| (path, metadata.modified().unwrap()))
        })
        .max_by_key(|&(_, modified_time)| modified_time)
        .map(|(path, _)| path);

    match latest_save {
        Some(path) => {
            println!("Serving file: {}", path.display());
            NamedFile::open(&path)
                .await
                .map_err(|e| MapError::NotFound(format!("Failed to open file: {}", e)))
        }
        None => {
            let msg = format!("No matching files found for pattern: {}", pattern);
            println!("{}", msg);
            Err(MapError::NotFound(msg))
        }
    }
}

#[options("/<_..>")]
fn all_options() {
    /* Intentionally left empty */
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new(
            "Access-Control-Allow-Origin",
            "https://satisfactory-calculator.com",
        ));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[rocket::main]
async fn main() -> rocket_anyhow::Result {
    // Parse command line arguments
    let args = Args::parse();

    // Validate save directory
    let path = Path::new(&args.save_dir);
    if !std::fs::metadata(&path)?.is_dir() {
        eprintln!("Save directory doesn' exist: {:?}", path);
        std::process::exit(1);
    };

    println!("Server starting with configuration:");
    println!("  Save directory: {}", args.save_dir);
    println!("  Port: {}", args.port);
    println!("\nEndpoints available:");
    println!("  - /map/<name>     : Serves the latest save file");

    let config = ServerConfig {
        save_dir: args.save_dir,
    };

    let figment = rocket::Config::figment().merge(("port", args.port));

    rocket::custom(figment)
        .attach(CORS)
        //.ignite()
        .mount("/", routes![serve_map, all_options])
        .manage(config)
        .launch()
        .await?;
    Ok(())
}
