use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use glob::glob;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::NamedFile;
use rocket::http::Header;
use rocket::response::content::RawHtml;
use rocket::response::Response;
use rocket::Request;
use rocket::State;
use serde::Deserialize;
mod rocket_anyhow;

#[macro_use]
extern crate rocket;

// Configuration structure
#[derive(Deserialize, Debug)]
struct Config {
    base_url: String,
    save_dir: String,
    port: u16,
}

impl Config {
    fn load() -> Result<Self> {
        // Try to load development config first
        if let Ok(config) = Self::load_from_file("config.dev.toml") {
            println!("Using development configuration from config.dev.toml");
            return Ok(config);
        }

        // Fall back to default config
        Self::load_from_file("config.toml")
            .context("Failed to load either config.dev.toml or config.toml")
    }

    fn load_from_file(path: &str) -> Result<Self> {
        let contents =
            fs::read_to_string(path).with_context(|| format!("Failed to read {}", path))?;

        toml::from_str(&contents).with_context(|| format!("Failed to parse {}", path))
    }
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
    base_url: String,
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

#[get("/map")]
fn map_index(config: &State<ServerConfig>) -> Result<RawHtml<String>, MapError> {
    let pattern = format!("{}/*.sav", config.save_dir);

    // Collect unique save names
    let mut save_names: HashSet<String> = HashSet::new();

    for entry in
        glob(&pattern).map_err(|e| MapError::BadRequest(format!("Invalid pattern: {}", e)))?
    {
        if let Ok(path) = entry {
            if let Some(file_name) = path.file_name() {
                if let Some(name) = file_name.to_string_lossy().split('_').next() {
                    save_names.insert(name.to_string());
                }
            }
        }
    }

    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Satisfactory Saves</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 2em auto; padding: 0 1em; }
        h1 { color: #333; }
        .save-list { list-style: none; padding: 0; }
        .save-list li { margin: 1em 0; }
        .save-list a {
            display: inline-block;
            padding: 0.5em 1em;
            background: #4CAF50;
            color: white;
            text-decoration: none;
            border-radius: 4px;
            transition: background 0.2s;
        }
        .save-list a:hover { background: #45a049; }
    </style>
</head>
<body>
    <h1>Available Satisfactory Saves</h1>
    <ul class="save-list">
"#,
    );

    let saves: Vec<_> = save_names.into_iter().collect();
    for save in saves {
        html.push_str(&format!(
            r#"        <li><a href="https://satisfactory-calculator.com/en/interactive-map?url={}/map/{}">{}</a></li>
"#,
            config.base_url, save, save
        ));
    }

    html.push_str(
        r#"    </ul>
</body>
</html>"#,
    );

    Ok(RawHtml(html))
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
    // Load configuration
    let config = Config::load()?;

    // Validate save directory
    let path = Path::new(&config.save_dir);
    if !std::fs::metadata(&path)?.is_dir() {
        eprintln!("Save directory doesn't exist: {:?}", path);
        std::process::exit(1);
    };

    println!("Server starting with configuration:");
    println!("  Save directory: {}", config.save_dir);
    println!("  Port: {}", config.port);
    println!("  Base URL: {}", config.base_url);
    println!("\nEndpoints available:");
    println!("  - /map/<name>     : Serves the latest save file");
    println!("  - /map            : Serves a list of available maps");

    let server_config = ServerConfig {
        save_dir: config.save_dir,
        base_url: config.base_url,
    };

    let figment = rocket::Config::figment().merge(("port", config.port));

    rocket::custom(figment)
        .attach(CORS)
        //.ignite()
        .mount("/", routes![serve_map, map_index, all_options])
        .manage(server_config)
        .launch()
        .await?;
    Ok(())
}
