# GT Mov

Personal movie library manager with automatic metadata from TMDB.

## Features

- **Folder Scan** — Import movies from local folders or external drives. Filenames must follow the pattern `Title (Year).ext`
- **TMDB Integration** — Automatic lookup of title, rating, description, poster and genres (German locale)
- **Sources** — Each scan path is stored as a source. Drive labels are auto-detected as alias
- **Filter & Search** — Filter by genre, source, rating, recency. Full-text search on title and year
- **Play** — Open movie files directly from the browser via the system default player
- **SSE Progress** — Folder scans stream real-time progress (processed/total) to the frontend
- **Single Binary** — Static files (HTML, CSS, JS, favicon) are embedded via `rust-embed`

## Tech Stack

| Layer    | Technology                          |
|----------|-------------------------------------|
| Backend  | Rust, Actix-Web 4, SQLx (SQLite)    |
| Frontend | Vanilla JS, CSS (dark theme)        |
| API      | TMDB via `tmdb-api` crate           |
| DB       | SQLite (auto-created `movies.db`)   |

## Setup

```sh
# .env
TMDB_API_KEY=your_api_key
# DATABASE_URL=sqlite:movies.db  (optional, default)
# PORT=8080 (optional, default)
```

```sh
cargo build --release
./target/release/gtmov
```

## File Naming Convention

```
Movie Title (2024).mp4
Another_Movie_(2020).mp4
```

Title and year are extracted from the filename and used for TMDB lookup.
