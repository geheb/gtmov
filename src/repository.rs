use crate::models::{MovieResponse, SourceResponse};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            alias TEXT,
            UNIQUE(path, alias)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS movies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            original_title TEXT,
            year INTEGER,
            rating REAL,
            image BLOB,
            description TEXT,
            genres TEXT,
            file_name TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            source_id INTEGER REFERENCES sources(id)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS genres (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}


pub async fn save_genres(pool: &SqlitePool, genres: &HashMap<u64, String>) -> Result<(), sqlx::Error> {
    for (id, name) in genres {
        sqlx::query("INSERT OR REPLACE INTO genres (id, name) VALUES (?, ?)")
            .bind(*id as i64)
            .bind(name)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn get_all_genres(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT name FROM genres ORDER BY name")
        .fetch_all(pool)
        .await
}

pub async fn get_all_movies(pool: &SqlitePool) -> Result<Vec<MovieResponse>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, original_title, year, rating, (image IS NOT NULL AND length(image) > 0) AS has_image, description, genres, file_name, created_at, source_id FROM movies ORDER BY title"
    )
    .fetch_all(pool)
    .await
}

pub async fn get_image(pool: &SqlitePool, id: i64) -> Result<Option<Vec<u8>>, sqlx::Error> {
    sqlx::query_scalar("SELECT image FROM movies WHERE id = ? AND image IS NOT NULL")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_full_file_path(pool: &SqlitePool, id: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT s.path || '/' || m.file_name FROM movies m JOIN sources s ON m.source_id = s.id WHERE m.id = ? AND m.file_name IS NOT NULL"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn file_exists_in_db(pool: &SqlitePool, file_name: &str, source_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) > 0 FROM movies WHERE file_name = ? AND source_id = ?")
        .bind(file_name)
        .bind(source_id)
        .fetch_one(pool)
        .await
}

pub async fn insert_movie(
    pool: &SqlitePool,
    movie: &crate::models::TmdbMovie,
    file_name: &str,
    genres: &Option<String>,
    source_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO movies (title, original_title, year, rating, description, file_name, image, genres, source_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&movie.title)
        .bind(&movie.original_title)
        .bind(movie.year)
        .bind(movie.rating)
        .bind(&movie.description)
        .bind(file_name)
        .bind(&movie.image)
        .bind(genres)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_or_create_source(pool: &SqlitePool, path: &str, alias: Option<&str>) -> Result<i64, sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO sources (path, alias) VALUES (?, ?)")
        .bind(path)
        .bind(alias)
        .execute(pool)
        .await?;

    sqlx::query_scalar("SELECT id FROM sources WHERE path = ?")
        .bind(path)
        .fetch_one(pool)
        .await
}

pub async fn get_all_sources(pool: &SqlitePool) -> Result<Vec<SourceResponse>, sqlx::Error> {
    sqlx::query_as("SELECT id, path, alias FROM sources ORDER BY path")
        .fetch_all(pool)
        .await
}
