use crate::models::TmdbMovie;
use chrono::Datelike;
use std::collections::HashMap;
use tmdb_api::client::reqwest::ReqwestExecutor;
use tmdb_api::client::Client;
use tmdb_api::genre::list::GenreList;
use tmdb_api::movie::search::MovieSearch;
use tmdb_api::prelude::Command;

pub async fn fetch_genres(client: &Client<ReqwestExecutor>) -> HashMap<u64, String> {
    let cmd = GenreList::movie();
    cmd.execute(client)
        .await
        .map(|res| res.into_iter().map(|g| (g.id, g.name)).collect())
        .unwrap_or_default()
}

pub fn resolve_genres(ids: &[u64], genre_map: &HashMap<u64, String>) -> Option<Vec<String>> {
    if ids.is_empty() {
        return None;
    }

    let names: Vec<String> = ids
        .iter()
        .filter_map(|id| genre_map.get(id).cloned())
        .collect();

    if names.is_empty() { None } else { Some(names) }
}

pub async fn search_movie(
    client: &Client<ReqwestExecutor>,
    genres_map: &HashMap<u64, String>,
    title: &str,
    year: i64,
) -> Result<TmdbMovie, Box<dyn std::error::Error>> {
    let mut cmd = MovieSearch::new(title.to_string())
        .with_language(Some("de-DE".to_string()))
        .with_region(Some("DE".to_string()))
        .with_include_adult(true)
        .with_primary_release_year(Some(year as u16));

    let res = match cmd.execute(client).await {
        Ok(res) => res,
        Err(_) => return Ok(TmdbMovie { title: title.to_string(), year, rating: None, description: None, image: None, genres: None }),
    };

    // Retry without year if no results found
    let results = if res.results.is_empty() {
        cmd = MovieSearch::new(title.to_string())
            .with_language(Some("de-DE".to_string()))
            .with_region(Some("DE".to_string()))
            .with_include_adult(true);

        match cmd.execute(client).await {
            Ok(res) => res.results,
            Err(_) => vec![],
        }
    } else {
        res.results
    };

    let title_lower = title.to_lowercase();
    let first = results.iter()
        .find(|m| m.inner.title.to_lowercase() == title_lower)
        .or_else(|| results.first())
        .cloned();

    match first.as_ref() {
        Some(m) => {
            let movie_year = m.inner.release_date
                .map(|d| d.year() as i64)
                .unwrap_or(year);

            let image = match m.inner.poster_path {
                Some(ref poster_path) => {
                    let url = format!("https://image.tmdb.org/t/p/w500{}", poster_path);
                    match reqwest::get(&url).await {
                        Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
                        _ => None,
                    }
                }
                None => None,
            };

            let genres = resolve_genres(&m.genre_ids, genres_map);

            Ok(TmdbMovie {
                title: m.inner.title.clone(),
                year: movie_year,
                rating: Some(m.inner.vote_average),
                description: Some(m.inner.overview.clone()),
                image,
                genres,
            })
        }
        None => Ok(TmdbMovie { title: title.to_string(), year, rating: None, description: None, image: None, genres: None }),
    }
}
