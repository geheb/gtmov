mod api;
mod models;
mod repository;
mod tmdb;

use actix_cors::Cors;
use actix_web::middleware::DefaultHeaders;
use actix_web::{web, App, HttpServer};
use sqlx::sqlite::SqlitePoolOptions;
use models::AppState;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:movies.db".to_string());

    let tmdb_api_key = std::env::var("TMDB_API_KEY")
        .expect("TMDB_API_KEY must be set in .env");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            database_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()
                .expect("Invalid DATABASE_URL")
                .create_if_missing(true),
        )
        .await
        .expect("Failed to create database pool");

    repository::init_db(&pool).await.expect("Failed to initialize database");

    let app_state = web::Data::new(AppState { db: pool, tmdb_api_key });

    let addr_bind = format!("127.0.0.1:{port}");
    println!("Server starting at http://{addr_bind}");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&format!("http://127.0.0.1:{port}"))
            .allowed_methods(vec!["GET", "POST", "DELETE"])
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(
                DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-XSS-Protection", "1; mode=block"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Content-Security-Policy", "default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self' 'unsafe-inline'"))
            )
            .app_data(app_state.clone())
            .service(api::get_version)
            .service(api::get_genres)
            .service(api::get_movies)
            .service(api::get_movie_image)
            .service(api::populate_movies)
            .service(api::open_movie)
            .service(api::get_sources)
            .service(api::delete_source)
            .route("/", web::get().to(api::serve_index))
            .route("/{path:.*}", web::get().to(api::serve_embedded))
    })
    .bind(addr_bind)?
    .run()
    .await
}
