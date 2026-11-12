//! Le catalogue complet des plateformes, par l'API de TMDB (The Movie Database) :
//! tous les films et séries disponibles en abonnement en France sur Netflix, Prime
//! Video, Disney+ et HBO Max, avec leur affiche, leur résumé en français, leurs
//! acteurs, leur année et leurs plateformes (données de disponibilité JustWatch).
//!
//! La clé se donne par l'environnement : `TMDB_TOKEN` (le « jeton d'accès en lecture »,
//! recommandé) ou `TMDB_API_KEY` (la clé v3). Sans clé, CineShuffle utilise sa sélection
//! embarquée. Les réponses sont gardées en mémoire quelques heures.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use cineshuffle::POPULAR;
use cineshuffle::catalog::{
    Genre, Kind, Platform, Profile, Rng, Suggestion, Title, hue_for, recommend,
};
use paquets::serde_json::Value;

/// L'API de TMDB (`TMDB_API_URL` la remplace, pour les tests).
const API: &str = "https://api.themoviedb.org/3";
/// Les images de TMDB (affiches en 342 pixels de large).
const IMAGES: &str = "https://image.tmdb.org/t/p/w342";
/// Le pays des disponibilités.
const REGION: &str = "FR";
/// La langue des titres et des résumés.
const LANGUAGE: &str = "fr-FR";
/// TMDB ne sert pas de page au-delà de la 500e.
const MAX_PAGE: u32 = 500;
/// Un titre doit avoir quelques votes sur TMDB (écarte les fiches vides).
const MIN_VOTES: &str = "10";
/// La durée de vie des réponses gardées en mémoire.
const TTL: Duration = Duration::from_secs(6 * 3600);
/// Le nombre de réponses gardées au plus.
const CACHE_SIZE: usize = 4000;

/// Les identifiants TMDB des plateformes en France, utilisés si la liste des
/// fournisseurs ne peut pas être lue.
fn default_provider(p: Platform) -> u32 {
    match p {
        Platform::Netflix => 8,
        Platform::PrimeVideo => 119,
        Platform::DisneyPlus => 337,
        Platform::HboMax => 1899,
    }
}

/// Les noms sous lesquels TMDB connaît chaque plateforme.
fn provider_names(p: Platform) -> &'static [&'static str] {
    match p {
        Platform::Netflix => &["Netflix"],
        Platform::PrimeVideo => &["Amazon Prime Video", "Prime Video"],
        Platform::DisneyPlus => &["Disney Plus", "Disney+"],
        Platform::HboMax => &["HBO Max", "Max"],
    }
}

enum Auth {
    /// Le jeton d'accès en lecture (API v4), dans l'en-tête `Authorization`.
    Bearer(String),
    /// La clé v3, dans l'adresse.
    Key(String),
}

/// Le client TMDB.
pub struct Tmdb {
    http: reqwest::Client,
    api: String,
    auth: Auth,
    cache: Mutex<HashMap<String, (Instant, Arc<Value>)>>,
    providers: tokio::sync::OnceCell<HashMap<Platform, u32>>,
}

/// Le client TMDB de l'application, si une clé est configurée.
pub fn client() -> Option<Arc<Tmdb>> {
    static CLIENT: OnceLock<Option<Arc<Tmdb>>> = OnceLock::new();
    CLIENT
        .get_or_init(|| Tmdb::from_env().map(Arc::new))
        .clone()
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

/// `film` → `movie`, `serie` → `tv`.
fn path(kind: Kind) -> &'static str {
    match kind {
        Kind::Film => "movie",
        Kind::Serie => "tv",
    }
}

impl Tmdb {
    /// Le client, avec la clé de l'environnement (`None` sans clé).
    pub fn from_env() -> Option<Tmdb> {
        let auth = match (env("TMDB_TOKEN"), env("TMDB_API_KEY")) {
            (Some(token), _) => Auth::Bearer(token),
            (None, Some(key)) => Auth::Key(key),
            (None, None) => return None,
        };
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("CineShuffle/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()?;
        Some(Tmdb {
            http,
            api: env("TMDB_API_URL").unwrap_or_else(|| API.to_owned()),
            auth,
            cache: Mutex::new(HashMap::new()),
            providers: tokio::sync::OnceCell::new(),
        })
    }

    /// `GET {API}{path}?{params}`, gardé en mémoire.
    async fn get(&self, path: &str, params: &[(&str, String)]) -> Result<Arc<Value>, String> {
        let mut query: Vec<(&str, String)> = params.to_vec();
        query.sort();
        let cache_key = format!(
            "{path}?{}",
            query
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        );
        if let Ok(cache) = self.cache.lock()
            && let Some((at, value)) = cache.get(&cache_key)
            && at.elapsed() < TTL
        {
            return Ok(value.clone());
        }
        let mut request = self.http.get(format!("{}{path}", self.api)).query(&query);
        request = match &self.auth {
            Auth::Bearer(token) => request.bearer_auth(token),
            Auth::Key(key) => request.query(&[("api_key", key)]),
        };
        let response = request.send().await.map_err(|e| format!("TMDB : {e}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("TMDB : {path} a répondu {status}"));
        }
        let value: Value = response.json().await.map_err(|e| format!("TMDB : {e}"))?;
        let value = Arc::new(value);
        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() >= CACHE_SIZE {
                cache.retain(|_, (at, _)| at.elapsed() < TTL);
                if cache.len() >= CACHE_SIZE {
                    cache.clear();
                }
            }
            cache.insert(cache_key, (Instant::now(), value.clone()));
        }
        Ok(value)
    }

    /// Les identifiants TMDB des quatre plateformes en France.
    async fn provider_ids(&self) -> &HashMap<Platform, u32> {
        self.providers
            .get_or_init(|| async {
                let mut ids: HashMap<Platform, u32> = Platform::ALL
                    .into_iter()
                    .map(|p| (p, default_provider(p)))
                    .collect();
                let params = [
                    ("watch_region", REGION.to_owned()),
                    ("language", LANGUAGE.to_owned()),
                ];
                if let Ok(list) = self.get("/watch/providers/movie", &params).await {
                    for p in Platform::ALL {
                        let found = list["results"].as_array().and_then(|all| {
                            provider_names(p).iter().find_map(|name| {
                                all.iter()
                                    .find(|r| r["provider_name"].as_str() == Some(name))
                                    .and_then(|r| r["provider_id"].as_u64())
                            })
                        });
                        if let Some(id) = found {
                            ids.insert(p, id as u32);
                        }
                    }
                }
                ids
            })
            .await
    }

    /// La plateforme d'un identifiant de fournisseur TMDB.
    async fn platform_of(&self, provider: u64) -> Option<Platform> {
        let ids = self.provider_ids().await;
        Platform::ALL
            .into_iter()
            .find(|p| ids.get(p).map(|&id| u64::from(id)) == Some(provider))
    }

    /// Une page des titres de `platforms`, les plus populaires d'abord, avec le nombre de
    /// pages et de titres. Les titres n'ont ni acteurs ni durée (voir `details`).
    pub async fn discover(
        &self,
        kind: Kind,
        platforms: &[Platform],
        page: u32,
        genres: &[u32],
    ) -> Result<(Vec<Title>, u32, u64), String> {
        let ids = self.provider_ids().await;
        let providers = platforms
            .iter()
            .filter_map(|p| ids.get(p))
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("|");
        let mut params = vec![
            ("watch_region", REGION.to_owned()),
            ("with_watch_providers", providers),
            ("with_watch_monetization_types", "flatrate".to_owned()),
            ("language", LANGUAGE.to_owned()),
            ("sort_by", "popularity.desc".to_owned()),
            ("include_adult", "false".to_owned()),
            ("vote_count.gte", MIN_VOTES.to_owned()),
            ("page", page.clamp(1, MAX_PAGE).to_string()),
        ];
        if !genres.is_empty() {
            params.push((
                "with_genres",
                genres
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join("|"),
            ));
        }
        let body = self
            .get(&format!("/discover/{}", path(kind)), &params)
            .await?;
        // Avec une seule plateforme demandée, chaque titre y est.
        let known: Vec<Platform> = if platforms.len() == 1 {
            platforms.to_vec()
        } else {
            Vec::new()
        };
        let titles = body["results"]
            .as_array()
            .map(|all| {
                all.iter()
                    .filter_map(|v| title_from(kind, v, known.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let pages = body["total_pages"]
            .as_u64()
            .unwrap_or(0)
            .min(u64::from(MAX_PAGE)) as u32;
        let results = body["total_results"].as_u64().unwrap_or(0);
        Ok((titles, pages, results))
    }

    /// Un titre complet : affiche, résumé, acteurs, durée et plateformes en France.
    pub async fn details(&self, kind: Kind, id: u64) -> Result<Title, String> {
        let params = [
            ("language", LANGUAGE.to_owned()),
            ("append_to_response", "credits,watch/providers".to_owned()),
        ];
        let body = self.get(&format!("/{}/{id}", path(kind)), &params).await?;
        let mut platforms = Vec::new();
        if let Some(offers) = body["watch/providers"]["results"][REGION]["flatrate"].as_array() {
            for offer in offers {
                if let Some(p) = match offer["provider_id"].as_u64() {
                    Some(provider) => self.platform_of(provider).await,
                    None => None,
                } && !platforms.contains(&p)
                {
                    platforms.push(p);
                }
            }
        }
        platforms.sort_by_key(|p| Platform::ALL.iter().position(|q| q == p));
        title_from(kind, &body, platforms).ok_or_else(|| "TMDB : fiche incomplète".to_owned())
    }

    /// Le titre d'une clé `movie:603` ou `tv:1399`.
    pub async fn title(&self, key: &str) -> Result<Title, String> {
        let (kind, id) = parse_key(key).ok_or_else(|| "clé inconnue".to_owned())?;
        self.details(kind, id).await
    }

    /// Complète des titres (acteurs, durée, plateformes), en parallèle. Un titre dont la
    /// fiche ne répond pas est gardé tel quel.
    async fn complete(self: &Arc<Self>, titles: Vec<Title>) -> Vec<Title> {
        let mut set = tokio::task::JoinSet::new();
        for (i, t) in titles.iter().enumerate() {
            let Some((kind, id)) = parse_key(&t.key) else {
                continue;
            };
            let me = self.clone();
            set.spawn(async move { (i, me.details(kind, id).await) });
        }
        let mut full = titles;
        while let Some(done) = set.join_next().await {
            if let Ok((i, Ok(mut detailed))) = done {
                if detailed.platforms.is_empty() {
                    detailed.platforms = full[i].platforms.clone();
                }
                full[i] = detailed;
            }
        }
        full
    }

    /// `n` titres tirés au sort dans tout le catalogue de `platform` (toutes : `None`)
    /// et de `kind`. La même graine donne le même tirage (tant que les catalogues ne
    /// changent pas).
    pub async fn picks(
        self: &Arc<Self>,
        seed: u64,
        platform: Option<Platform>,
        kind: Option<Kind>,
        n: usize,
    ) -> Result<Vec<Title>, String> {
        let platforms = platform.map_or_else(|| Platform::ALL.to_vec(), |p| vec![p]);
        let kinds = kind.map_or_else(|| vec![Kind::Film, Kind::Serie], |k| vec![k]);
        let mut totals = Vec::new();
        for k in kinds {
            let (_, pages, results) = self.discover(k, &platforms, 1, &[]).await?;
            if results > 0 && pages > 0 {
                totals.push((k, pages, results));
            }
        }
        let sum: u64 = totals.iter().map(|t| t.2).sum();
        if sum == 0 {
            return Ok(Vec::new());
        }
        let mut rng = Rng::new(seed ^ 0x7E1E_5EED);
        let mut chosen: Vec<Title> = Vec::new();
        let mut attempts = 0;
        while chosen.len() < n && attempts < n * 6 {
            attempts += 1;
            // Le format au prorata du nombre de titres, puis une page et un titre.
            let mut ticket = rng.next_u64() % sum;
            let &(k, pages, _) = totals
                .iter()
                .find(|t| {
                    if ticket < t.2 {
                        true
                    } else {
                        ticket -= t.2;
                        false
                    }
                })
                .unwrap_or(&totals[0]);
            let page = 1 + rng.below(pages as usize) as u32;
            let (list, _, _) = self.discover(k, &platforms, page, &[]).await?;
            if list.is_empty() {
                continue;
            }
            let t = &list[rng.below(list.len())];
            if t.poster.is_some() && !chosen.iter().any(|c| c.key == t.key) {
                chosen.push(t.clone());
            }
        }
        Ok(self.complete(chosen).await)
    }

    /// Les suggestions de `profile` : les titres de ses plateformes dans ses genres
    /// favoris, classés par le moteur de CineShuffle, puis complétés (acteurs, plateformes).
    pub async fn recommend(
        self: &Arc<Self>,
        profile: &Profile,
        n: usize,
    ) -> Result<Vec<Suggestion>, String> {
        let platforms = if profile.platforms.is_empty() {
            Platform::ALL.to_vec()
        } else {
            profile.platforms.clone()
        };
        let kinds = profile
            .kind
            .map_or_else(|| vec![Kind::Film, Kind::Serie], |k| vec![k]);
        let top = profile.top_genres(3);
        let mut lists = Vec::new();
        for &k in &kinds {
            let mut ids: Vec<u32> = top.iter().filter_map(|g| g.tmdb_id(k)).collect();
            ids.dedup();
            let mut list = Vec::new();
            for page in 1..=2 {
                let (titles, pages, _) = self.discover(k, &platforms, page, &ids).await?;
                list.extend(titles);
                if page >= pages {
                    break;
                }
            }
            lists.push(list);
        }
        // Films et séries en alternance : le rang de popularité compte dans le score.
        let mut candidates = Vec::new();
        let longest = lists.iter().map(Vec::len).max().unwrap_or(0);
        for i in 0..longest {
            for list in &lists {
                if let Some(t) = list.get(i) {
                    candidates.push(t.clone());
                }
            }
        }
        let found = recommend(&candidates, profile, n, POPULAR);
        let titles = self
            .complete(found.iter().map(|s| s.title.clone()).collect())
            .await;
        Ok(found
            .into_iter()
            .zip(titles)
            .map(|(s, title)| Suggestion { title, ..s })
            .collect())
    }

    /// Une page du catalogue de `platform` (20 titres) et le nombre de pages.
    pub async fn browse(
        &self,
        platform: Platform,
        kind: Kind,
        page: u32,
    ) -> Result<(Vec<Title>, u32), String> {
        let (titles, pages, _) = self.discover(kind, &[platform], page, &[]).await?;
        Ok((titles, pages))
    }
}

/// `movie:603` → `(Film, 603)`.
pub fn parse_key(key: &str) -> Option<(Kind, u64)> {
    let (kind, id) = key.split_once(':')?;
    let kind = match kind {
        "movie" => Kind::Film,
        "tv" => Kind::Serie,
        _ => return None,
    };
    Some((kind, id.parse().ok()?))
}

/// Un titre à partir d'une fiche TMDB (résultat de recherche ou fiche complète).
pub fn title_from(kind: Kind, v: &Value, platforms: Vec<Platform>) -> Option<Title> {
    let id = v["id"].as_u64()?;
    let key = format!("{}:{id}", path(kind));
    let name = match kind {
        Kind::Film => v["title"].as_str(),
        Kind::Serie => v["name"].as_str(),
    }?
    .to_owned();
    let date = match kind {
        Kind::Film => v["release_date"].as_str(),
        Kind::Serie => v["first_air_date"].as_str(),
    };
    let year = date.and_then(|d| d.get(..4)).and_then(|y| y.parse().ok());
    let mut genre_ids: Vec<u64> = v["genre_ids"]
        .as_array()
        .map(|ids| ids.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default();
    if let Some(genres) = v["genres"].as_array() {
        genre_ids.extend(genres.iter().filter_map(|g| g["id"].as_u64()));
    }
    let mut genres: Vec<Genre> = Vec::new();
    for id in genre_ids {
        for &g in Genre::from_tmdb(id as u32) {
            if !genres.contains(&g) {
                genres.push(g);
            }
        }
    }
    let length = match kind {
        Kind::Film => v["runtime"]
            .as_u64()
            .filter(|m| *m > 0)
            .map(|m| format!("{} h {:02}", m / 60, m % 60)),
        Kind::Serie => v["number_of_seasons"].as_u64().filter(|s| *s > 0).map(|s| {
            if s == 1 {
                "1 saison".to_owned()
            } else {
                format!("{s} saisons")
            }
        }),
    };
    let cast = v["credits"]["cast"]
        .as_array()
        .map(|people| {
            people
                .iter()
                .filter_map(|p| p["name"].as_str())
                .take(5)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let rating = v["vote_average"]
        .as_f64()
        .filter(|r| *r > 0.0)
        .map(|r| (r * 10.0).round() as f32 / 10.0);
    Some(Title {
        emoji: genres.first().map_or("🎬", |g| g.emoji()).to_owned(),
        hue: hue_for(&key),
        key,
        name,
        year,
        kind,
        platforms,
        genres,
        overview: v["overview"].as_str().unwrap_or_default().trim().to_owned(),
        poster: v["poster_path"]
            .as_str()
            .filter(|p| p.starts_with('/'))
            .map(|p| format!("{IMAGES}{p}")),
        cast,
        length,
        rating,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use paquets::serde_json::json;

    #[test]
    fn a_full_movie_page() {
        let v = json!({
            "id": 603,
            "title": "Matrix",
            "release_date": "1999-03-31",
            "overview": " Programmeur le jour, pirate la nuit… ",
            "poster_path": "/matrix.jpg",
            "genres": [{ "id": 28, "name": "Action" }, { "id": 878, "name": "Science-Fiction" }],
            "runtime": 136,
            "vote_average": 8.234,
            "credits": { "cast": [
                { "name": "Keanu Reeves" }, { "name": "Laurence Fishburne" },
                { "name": "Carrie-Anne Moss" }, { "name": "Hugo Weaving" },
                { "name": "Joe Pantoliano" }, { "name": "Marcus Chong" }
            ] }
        });
        let t = title_from(Kind::Film, &v, vec![Platform::Netflix]).unwrap();
        assert_eq!(t.key, "movie:603");
        assert_eq!(t.name, "Matrix");
        assert_eq!(t.year, Some(1999));
        assert_eq!(t.genres, [Genre::Action, Genre::Sf]);
        assert_eq!(t.length.as_deref(), Some("2 h 16"));
        assert_eq!(t.cast.len(), 5);
        assert_eq!(t.rating, Some(8.2));
        assert_eq!(t.overview, "Programmeur le jour, pirate la nuit…");
        assert_eq!(
            t.poster.as_deref(),
            Some("https://image.tmdb.org/t/p/w342/matrix.jpg")
        );
        assert_eq!(t.emoji, "💥");
        assert_eq!(t.meta(), "1999 · Film · 2 h 16");
    }

    #[test]
    fn a_series_from_a_search() {
        let v = json!({
            "id": 1399,
            "name": "Game of Thrones",
            "first_air_date": "2011-04-17",
            "genre_ids": [10765, 18, 10759],
            "poster_path": null,
            "overview": ""
        });
        let t = title_from(Kind::Serie, &v, Vec::new()).unwrap();
        assert_eq!(t.key, "tv:1399");
        assert_eq!(t.year, Some(2011));
        assert_eq!(
            t.genres,
            [
                Genre::Sf,
                Genre::Fantastique,
                Genre::Drame,
                Genre::Action,
                Genre::Aventure
            ]
        );
        assert_eq!(t.poster, None);
        assert!(t.cast.is_empty() && t.length.is_none());
        assert!(title_from(Kind::Serie, &json!({ "id": 1 }), Vec::new()).is_none());
    }

    #[test]
    fn keys() {
        assert_eq!(parse_key("movie:603"), Some((Kind::Film, 603)));
        assert_eq!(parse_key("tv:1399"), Some((Kind::Serie, 1399)));
        assert_eq!(parse_key("local:3"), None);
        assert_eq!(parse_key("movie:x"), None);
    }

    /// Avec une vraie clé (`TMDB_TOKEN` ou `TMDB_API_KEY`) : `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "appelle l'API TMDB"]
    async fn live() {
        let tmdb = client().expect("TMDB_TOKEN ou TMDB_API_KEY");
        let picks = tmdb.picks(42, None, None, 5).await.unwrap();
        assert_eq!(picks.len(), 5);
        for t in &picks {
            println!(
                "{} ({:?}) {:?} {:?} — {}",
                t.name,
                t.year,
                t.platforms,
                t.cast,
                t.poster.as_deref().unwrap_or("-")
            );
            assert!(t.poster.is_some());
        }
        let prime = tmdb
            .picks(7, Some(Platform::PrimeVideo), Some(Kind::Serie), 5)
            .await
            .unwrap();
        assert!(prime.iter().all(|t| t.kind == Kind::Serie));
        let (page, pages) = tmdb.browse(Platform::Netflix, Kind::Film, 1).await.unwrap();
        println!("Netflix : {pages} pages de films");
        assert_eq!(page.len(), 20);
        assert!(pages > 10);
    }
}
