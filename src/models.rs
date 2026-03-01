use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;


#[derive(Embed)]
#[folder = "static/"]
pub struct StaticFiles;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MovieResponse {
    pub id: i64,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i64>,
    pub rating: Option<f64>,
    pub has_image: bool,
    pub description: Option<String>,
    pub genres: Option<String>,
    pub file_name: Option<String>,
    pub created_at: String,
    pub source_id: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SourceResponse {
    pub id: i64,
    pub path: String,
    pub alias: Option<String>,
}

#[derive(Deserialize)]
pub struct PopulateQuery {
    pub path: String,
}

pub struct AppState {
    pub db: SqlitePool,
    pub tmdb_api_key: String,
}

pub struct TmdbMovie {
    pub title: String,
    pub original_title: Option<String>,
    pub year: i32,
    pub rating: Option<f64>,
    pub description: Option<String>,
    pub image: Option<Vec<u8>>,
    pub genres: Option<Vec<String>>,
}
