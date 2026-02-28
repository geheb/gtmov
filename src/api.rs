use actix_web::{get, post, web, HttpResponse, Responder};
use bytes::Bytes;
use regex::Regex;
use std::path::{Path, PathBuf};
use tmdb_api::client::reqwest::ReqwestExecutor;
use tmdb_api::client::Client;
use tokio::sync::mpsc;

use crate::models::{AppState, PopulateQuery, StaticFiles};
use crate::repository;
use crate::tmdb;

#[get("/api/genres")]
pub async fn get_genres(data: web::Data<AppState>) -> impl Responder {
    match repository::get_all_genres(&data.db).await {
        Ok(genres) => HttpResponse::Ok().json(genres),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to fetch genres: {}", e)
        })),
    }
}

#[get("/api/movies")]
pub async fn get_movies(data: web::Data<AppState>) -> impl Responder {
    match repository::get_all_movies(&data.db).await {
        Ok(movies) => HttpResponse::Ok().json(movies),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to fetch movies: {}", e)
        })),
    }
}

#[get("/api/movies/image/{id}.jpg")]
pub async fn get_movie_image(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let id = path.into_inner();

    match repository::get_image(&data.db, id).await {
        Ok(Some(image_data)) if !image_data.is_empty() => HttpResponse::Ok()
            .content_type("image/jpeg")
            .body(image_data),
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "No image found for this movie"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to fetch image: {}", e)
        })),
    }
}

#[get("/api/movies/play/{id}")]
pub async fn open_movie(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let id = path.into_inner();

    match repository::get_full_file_path(&data.db, id).await {
        Ok(Some(path)) => {
            match open::that_detached(&path) {
                Ok(_) => HttpResponse::Accepted().body(""),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to open file '{}': {}", path, e)
                })),
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Movie not found or has no file path"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to open file: {}", e)
        })),
    }
}

#[post("/api/populate")]
pub async fn populate_movies(
    data: web::Data<AppState>,
    query: web::Query<PopulateQuery>,
) -> impl Responder {
    let scan_path = query.path.clone();
    let path = PathBuf::from(&scan_path);

    if !path.exists() || !path.is_dir() {
        return HttpResponse::BadRequest()
            .content_type("text/event-stream")
            .body(format!("data: {{\"error\":\"Path '{}' does not exist or is not a directory\"}}\n\n", scan_path));
    }

    let db = data.db.clone();
    let tmdb_client = Client::<ReqwestExecutor>::new(data.tmdb_api_key.clone());

    let genres = tmdb::fetch_genres(&tmdb_client).await;
    if let Err(e) = repository::save_genres(&db, &genres).await {
        return HttpResponse::InternalServerError()
            .content_type("text/event-stream")
            .body(format!("data: {{\"error\":\"Failed to save genres: {}\"}}\n\n", e));
    }

    let volume_label = get_volume_label(&path);
    let source_id = match repository::get_or_create_source(&db, &scan_path, volume_label.as_deref()).await {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .content_type("text/event-stream")
                .body(format!("data: {{\"error\":\"Failed to create source: {}\"}}\n\n", e));
        }
    };

    let re = Regex::new(r"^(.+)\((\d{4})\)\.\w+$").unwrap();

    let mut files: Vec<(String, String, i64)> = Vec::new();
    collect_files(&path, &path, &re, &mut files).await;
    let total = files.len() as u32;

    let (tx, mut rx) = mpsc::channel::<String>(32);

    actix_web::rt::spawn(async move {
        let send = |msg: String| {
            let tx = tx.clone();
            async move { let _ = tx.send(msg).await; }
        };

        send(format!("{{\"total\":{}}}", total)).await;

        // Process files with progress
        let mut added: u32 = 0;
        let mut skipped: u32 = 0;
        let mut processed: u32 = 0;

        for (filename, title, year) in &files {
            processed += 1;

            let exists = match repository::file_exists_in_db(&db, filename, source_id).await {
                Ok(v) => v,
                Err(_) => { continue; }
            };

            if exists {
                skipped += 1;
            } else {
                match tmdb::search_movie(&tmdb_client, &genres, &title, *year).await {
                    Ok(movie) => {
                        let genre_str = movie.genres.as_ref().map(|v| v.join(", "));
                        if repository::insert_movie(&db, &movie, filename, &genre_str, source_id).await.is_ok() {
                            added += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("TMDB lookup failed for '{}': {}", title, e);
                    }
                }
            }

            send(format!("{{\"processed\":{},\"total\":{},\"added\":{},\"skipped\":{},\"current\":\"{}\"}}", processed, total, added, skipped, title.replace('"', "\\\""))).await;
        }

        send(format!("{{\"done\":true,\"added\":{},\"skipped\":{}}}", added, skipped)).await;
    });

    let stream = async_stream::stream! {
        while let Some(msg) = rx.recv().await {
            yield Ok::<Bytes, actix_web::Error>(Bytes::from(format!("data: {}\n\n", msg)));
        }
    };

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(stream)
}

fn get_volume_label(_path: &Path) -> Option<String> {
    #[cfg(target_os = "linux")]
    { None }

    #[cfg(not(target_os = "linux"))]
    {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        disks.iter()
            .find(|d| _path.starts_with(d.mount_point()))
            .and_then(|d| {
                let name = d.name().to_string_lossy().to_string();
                if name.is_empty() { None } else { Some(name) }
            })
    }
}

async fn collect_files(root: &Path, dir: &Path, re: &Regex, files: &mut Vec<(String, String, i64)>) {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Skipping directory '{}': {}", dir.display(), e);
            return;
        }
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            Box::pin(collect_files(root, &path, re, files)).await;
        } else if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if let Some(caps) = re.captures(filename) {
                let title = caps[1].replace('_', " ").trim().to_string();
                let year: i64 = caps[2].parse().unwrap_or(0);
                let rel_path = path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                files.push((rel_path, title, year));
            } else {
                println!("Skipping unmatched file: {}", filename);
            }
        }
    }
}

#[get("/api/sources")]
pub async fn get_sources(data: web::Data<AppState>) -> impl Responder {
    match repository::get_all_sources(&data.db).await {
        Ok(sources) => HttpResponse::Ok().json(sources),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to fetch sources: {}", e)
        })),
    }
}


pub async fn serve_embedded(path: web::Path<String>) -> impl Responder {
    let path = path.into_inner();
    let file_path = if path.is_empty() { "index.html" } else { &path };

    match StaticFiles::get(file_path) {
        Some(content) => {
            let mime = mime_guess::from_path(file_path).first_or_octet_stream();
            HttpResponse::Ok()
                .content_type(mime.as_ref())
                .body(content.data.into_owned())
        }
        None => match StaticFiles::get("index.html") {
            Some(content) => HttpResponse::Ok()
                .content_type("text/html")
                .body(content.data.into_owned()),
            None => HttpResponse::NotFound().body("Not found"),
        },
    }
}

pub async fn serve_index() -> impl Responder {
    match StaticFiles::get("index.html") {
        Some(content) => HttpResponse::Ok()
            .content_type("text/html")
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("Not found"),
    }
}
