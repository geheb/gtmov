let allMovies = [];
let activeGenre = 'all';
let activeFilter = 'all';
let activeSource = 'all';
let searchQuery = '';

// DOM elements
const moviesGrid = document.getElementById('moviesGrid');
const searchInput = document.getElementById('searchInput');
const genreList = document.getElementById('genreList');
const modalOverlay = document.getElementById('modalOverlay');
const modalClose = document.getElementById('modalClose');
const importOverlay = document.getElementById('importOverlay');
const importClose = document.getElementById('importClose');
const hamburgerBtn = document.getElementById('hamburgerBtn');
const populateBtn = document.getElementById('populateBtn');
const populatePath = document.getElementById('populatePath');
const sourceSelect = document.getElementById('sourceSelect');
const toast = document.getElementById('toast');

// Fetch and display movies
async function loadMovies() {
    try {
        const res = await fetch('/api/movies');
        if (!res.ok) throw new Error('Failed to load movies');
        allMovies = await res.json();
        renderMovies();
    } catch (err) {
        showToast(err.message, 'error');
    }
}

// Render movies based on current filters
function renderMovies() {
    const threeYearsAgo = new Date().getFullYear() - 3;
    const filtered = allMovies.filter(movie => {
        // Source filter
        if (activeSource !== 'all') {
            if (movie.source_id !== parseInt(activeSource)) return false;
        }

        // Genre filter
        if (activeGenre !== 'all') {
            if (!movie.genres || !movie.genres.toLowerCase().includes(activeGenre.toLowerCase())) {
                return false;
            }
        }

        // Search filter
        if (searchQuery) {
            const q = searchQuery.toLowerCase();
            const matchTitle = movie.title.toLowerCase().includes(q);
            const matchYear = movie.year && movie.year.toString().includes(q);
            if (!matchTitle && !matchYear) return false;
        }

        // Tag filters
        const rating = movie.rating?.toFixed(1) ?? 0;
        if (activeFilter === 'rated' && rating < 7.0) return false;
        if (activeFilter === 'unrated' && rating >= 7.0) return false;
        if (activeFilter === 'recent') {
            if (!movie.year || movie.year <= threeYearsAgo) return false;
        }

        return true;
    });

    document.getElementById('movieCount').textContent = filtered.length + ' Movies';

    if (filtered.length === 0) {
        moviesGrid.innerHTML = `
            <div class="empty-state">
                <div class="icon">&#127916;</div>
                <h2>No movies found</h2>
                <p>Try adjusting your filters or import movies using the sidebar.</p>
            </div>
        `;
        return;
    }

    moviesGrid.innerHTML = filtered.map(movie => `
        <div class="movie-card" onclick="showDetail(${movie.id})">
            <div class="movie-poster">
                ${movie.has_image
                    ? `<img src="/api/movies/image/${movie.id}.jpg" alt="${escapeHtml(movie.title)}">`
                    : `<div class="movie-poster-placeholder">
                        <span class="icon">&#127916;</span>
                        ${escapeHtml(movie.title)}
                       </div>`
                }
            </div>
            <div class="movie-info">
                <div class="movie-title" title="${escapeHtml(movie.title)}">${escapeHtml(movie.title)}</div>
                <div class="movie-meta">
                    <span class="movie-year">${movie.year || '—'}</span>
                    ${movie.rating ? `<span class="movie-rating">&#9733; ${movie.rating.toFixed(1)}</span>` : ''}
                </div>
                ${movie.genres ? `<span class="movie-genre-tag">${escapeHtml(movie.genres)}</span>` : ''}
            </div>
        </div>
    `).join('');
}

// Show movie detail modal
function showDetail(id) {
    const movie = allMovies.find(m => m.id === id);
    if (!movie) return;

    document.getElementById('modalTitle').textContent = movie.title;
    document.getElementById('modalYear').textContent = movie.year || 'Unknown year';

    const ratingEl = document.getElementById('modalRating');
    if (movie.rating) {
        ratingEl.textContent = `\u2605 ${movie.rating.toFixed(1)}`;
        ratingEl.style.display = '';
    } else {
        ratingEl.style.display = 'none';
    }

    // Genre tags
    const genreEl = document.getElementById('modalGenre');
    if (movie.genres) {
        genreEl.innerHTML = movie.genres.split(',').map(g =>
            `<span class="genre-tag">${escapeHtml(g.trim())}</span>`
        ).join('');
    } else {
        genreEl.innerHTML = '';
    }

    // Description
    const descEl = document.getElementById('modalDescription');
    descEl.textContent = movie.description || 'No description available.';

    // Image
    const imgEl = document.getElementById('modalImage');
    const placeholderEl = document.getElementById('modalPlaceholder');
    if (movie.has_image) {
        imgEl.src = `/api/movies/image/${movie.id}.jpg`;
        imgEl.style.display = '';
        placeholderEl.style.display = 'none';
    } else {
        imgEl.style.display = 'none';
        placeholderEl.style.display = '';
    }

    // Play button
    const playEl = document.getElementById('modalPlay');
    if (movie.file_name) {
        playEl.dataset.movieId = movie.id;
        playEl.classList.remove('hidden');
    } else {
        playEl.classList.add('hidden');
    }

    modalOverlay.classList.add('active');
}

// Open movie file
async function openMovie() {
    const playBtn = document.getElementById('modalPlay');
    const id = playBtn.dataset.movieId;
    if (!id) return;

    try {
        const res = await fetch(`/api/movies/play/${id}`);
        if (!res.ok) {
            const data = await res.json();
            throw new Error(data.error || 'Failed to open movie');
        }
    } catch (err) {
        showToast(err.message, 'error');
    }
}

// Close modal
function closeModal() {
    modalOverlay.classList.remove('active');
}

// Toast notification
function showToast(message, type = 'success') {
    toast.textContent = message;
    toast.className = 'toast show ' + type;
    setTimeout(() => {
        toast.className = 'toast';
    }, 3000);
}

// Escape HTML to prevent XSS
function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

// Populate from folder
async function populateFromFolder() {
    const path = populatePath.value.trim();
    if (!path) {
        showToast('Please enter a folder path', 'error');
        return;
    }

    const progressEl = document.getElementById('importProgress');
    const progressFill = document.getElementById('progressFill');
    const progressText = document.getElementById('progressText');

    populateBtn.disabled = true;
    populateBtn.innerHTML = '<span class="spinner"></span> Scanning...';
    progressEl.classList.remove('hidden');
    progressFill.style.width = '0%';
    progressText.textContent = 'Collecting files...';

    try {
        const res = await fetch(`/api/populate?path=${encodeURIComponent(path)}`, { method: 'POST' });
        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            buffer += decoder.decode(value, { stream: true });
            const lines = buffer.split('\n');
            buffer = lines.pop();

            for (const line of lines) {
                if (!line.startsWith('data: ')) continue;
                const data = JSON.parse(line.slice(6));

                if (data.error) {
                    throw new Error(data.error);
                }

                if (data.total !== undefined && !data.processed) {
                    progressText.textContent = `Found ${data.total} videos`;
                }

                if (data.processed !== undefined) {
                    const pct = data.total > 0 ? (data.processed / data.total * 100) : 0;
                    progressFill.style.width = pct + '%';
                    progressText.textContent = `${data.processed} / ${data.total} — ${escapeHtml(data.current)}`;
                }

                if (data.done) {
                    importOverlay.classList.remove('active');
                    showToast(`Scan complete. Added: ${data.added}, Skipped: ${data.skipped}`, 'success');
                    await loadSources();
                    await loadGenres();
                    await loadMovies();
                }
            }
        }
    } catch (err) {
        showToast(err.message, 'error');
    } finally {
        populateBtn.disabled = false;
        populateBtn.textContent = 'Scan Folder';
        progressEl.classList.add('hidden');
    }
}

// Event listeners
searchInput.addEventListener('input', (e) => {
    searchQuery = e.target.value;
    renderMovies();
});

genreList.addEventListener('click', (e) => {
    const item = e.target.closest('.genre-item');
    if (!item) return;

    genreList.querySelectorAll('.genre-item').forEach(el => el.classList.remove('active'));
    item.classList.add('active');
    activeGenre = item.dataset.genre;
    renderMovies();
});

document.querySelectorAll('.filter-tag').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.filter-tag').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        activeFilter = btn.dataset.filter;
        renderMovies();
    });
});

modalClose.addEventListener('click', closeModal);
modalOverlay.addEventListener('click', (e) => {
    if (e.target === modalOverlay) closeModal();
});

hamburgerBtn.addEventListener('click', () => {
    importOverlay.classList.add('active');
    populatePath.focus();
});

importClose.addEventListener('click', () => {
    importOverlay.classList.remove('active');
});

importOverlay.addEventListener('click', (e) => {
    if (e.target === importOverlay) importOverlay.classList.remove('active');
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
        closeModal();
        importOverlay.classList.remove('active');
    }
});

populateBtn.addEventListener('click', populateFromFolder);
populatePath.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') populateFromFolder();
});

// Load sources from API
async function loadSources() {
    try {
        const res = await fetch('/api/sources');
        if (!res.ok) throw new Error('Failed to load sources');
        const sources = await res.json();

        const options = sources.map(s => {
            const label = s.alias || s.path.split(/[/\\]/).filter(Boolean).pop() || s.path;
            return `<option value="${s.id}">${escapeHtml(label)}</option>`;
        }).join('');

        sourceSelect.innerHTML = options;
        if (sources.length > 0 && activeSource === 'all') {
            activeSource = String(sources[0].id);
            sourceSelect.value = activeSource;
        }
    } catch (err) {
        showToast(err.message, 'error');
    }
}

sourceSelect.addEventListener('change', () => {
    activeSource = sourceSelect.value;
    loadMovies();
});


// Load genres from API
async function loadGenres() {
    try {
        const res = await fetch('/api/genres');
        if (!res.ok) throw new Error('Failed to load genres');
        const genres = await res.json();

        const items = genres.map(g =>
            `<li class="genre-item" data-genre="${escapeHtml(g)}">${escapeHtml(g)}</li>`
        ).join('');

        genreList.innerHTML = `<li class="genre-item active" data-genre="all">All Movies</li>${items}`;
    } catch (err) {
        showToast(err.message, 'error');
    }
}

// Initial load
async function init() {
    fetch('/api/version').then(r => r.json()).then(d => {
        document.title = `GT Mov v${d.version}`;
    });
    await loadSources();
    loadGenres();
    loadMovies();
}
init();
