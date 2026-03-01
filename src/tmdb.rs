use crate::models::TmdbMovie;
use chrono::Datelike;
use tmdb_api::movie::details::MovieDetails;
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
    year: i32,
) -> Result<TmdbMovie, Box<dyn std::error::Error>> {
    let cmd = MovieSearch::new(title.to_string())
        .with_language(Some("en".to_string()))
        .with_include_adult(true);

    let results = match cmd.execute(client).await {
        Ok(res) => res.results,
        Err(_) => return Ok(TmdbMovie { title: title.to_string(), original_title: None, year, rating: None, description: None, image: None, genres: None }),
    };

    let title_lower = title.to_lowercase();
    let first = results.iter()
        .find(|m| m.inner.title.to_lowercase() == title_lower && m.inner.release_date.map(|d|d.year()).unwrap_or(0i32) == year)
        .or_else(|| results.iter().find(|m|m.inner.title.to_lowercase() == title_lower))
        .or_else(|| results.first())
        .cloned();

    match first.as_ref() {
        Some(m) => {
            let movie_year = m.inner.release_date
                .map(|d| d.year())
                .unwrap_or(year);

            let mut title = m.inner.title.clone();
            let original_title = Some(m.inner.original_title.clone());
            let rating = Some(m.inner.vote_average);
            let mut description = Some(m.inner.overview.clone());
            let mut poster_path = m.inner.poster_path.clone();

            let cmd_details = MovieDetails::new(m.inner.id)
                .with_language(Some("de-DE".to_string()));
            match cmd_details.execute(client).await {
                Ok(res) => {
                    if !res.inner.title.is_empty() {
                        title = res.inner.title.clone();
                    }
                    if !res.inner.overview.is_empty() {
                        description = Some(res.inner.overview.clone());
                    }
                    if let Some(p) = res.inner.poster_path.filter(|s| !s.is_empty()) {
                        poster_path = Some(p);
                    }
                },
                Err(_) => {}
            }

            let image = match poster_path {
                Some(ref p) => {
                    let url = format!("https://image.tmdb.org/t/p/w500{}", p);
                    match reqwest::get(&url).await {
                        Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
                        _ => None,
                    }
                }
                None => None,
            };

            let genres = resolve_genres(&m.genre_ids, genres_map);

            Ok(TmdbMovie {
                title,
                original_title,
                year: movie_year,
                rating,
                description,
                image,
                genres,
            })
        }
        None => Ok(TmdbMovie { title: title.to_string(), original_title: None, year, rating: None, description: None, image: None, genres: None }),
    }
}
