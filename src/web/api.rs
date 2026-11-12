//! L'API JSON, utilisée par les îlots du site et par l'application iOS.
//!
//! | Route | Rôle |
//! |---|---|
//! | `GET /api/titles` | le catalogue |
//! | `GET /api/titles/:id` | un titre |
//! | `GET /api/picks?platform=&kind=&seed=` | 5 titres au hasard (la sélection du jour sans `seed`) |
//! | `POST /api/recommendations` | des suggestions selon des votes et des préférences |
//! | `GET /api/votes/stats` | les votes de la communauté, par titre |
//! | `POST /api/votes` | un vote (`liked`), ou son changement (`previous`) |
//! | `POST /api/proposals` | proposer un titre à ajouter au catalogue |

use std::time::{SystemTime, UNIX_EPOCH};

use paquets::prelude::*;
use reelpick::catalog::{self, Genre, Kind, PICKS, Platform, Profile, Title};

/// Les votes d'un titre.
#[model]
pub struct TitleStat {
    /// L'identifiant du titre.
    #[memoire(primary_key)]
    pub title: i64,
    /// Les « aimé ».
    pub likes: i64,
    /// Les « pas aimé ».
    pub dislikes: i64,
}

/// Un titre proposé par un visiteur.
#[model]
pub struct ProposalRow {
    /// L'identifiant, donné par la base.
    pub id: i64,
    /// Le titre proposé.
    pub title: String,
    /// La plateforme.
    pub platform: String,
}

/// Un titre, tel que l'API l'envoie.
#[data]
pub struct TitleJson {
    id: u32,
    name: String,
    year: u16,
    kind: String,
    platform: String,
    platform_label: String,
    genres: Vec<String>,
    genre_labels: Vec<String>,
    length: String,
    pitch: String,
    emoji: String,
    hue: u16,
}

impl From<&Title> for TitleJson {
    fn from(t: &Title) -> Self {
        TitleJson {
            id: t.id,
            name: t.name.to_owned(),
            year: t.year,
            kind: t.kind.slug().to_owned(),
            platform: t.platform.slug().to_owned(),
            platform_label: t.platform.label().to_owned(),
            genres: t.genres.iter().map(|g| g.slug().to_owned()).collect(),
            genre_labels: t.genres.iter().map(|g| g.label().to_owned()).collect(),
            length: t.length.to_owned(),
            pitch: t.pitch.to_owned(),
            emoji: t.emoji.to_owned(),
            hue: t.hue,
        }
    }
}

/// La graine de la sélection du jour.
pub fn today_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    catalog::day_seed(now)
}

/// Le catalogue complet.
pub fn all_titles() -> Vec<TitleJson> {
    catalog::CATALOG.iter().map(TitleJson::from).collect()
}

/// Le catalogue en JSON lisible (le fichier embarqué par l'application iOS).
pub fn catalog_json() -> String {
    paquets::serde_json::to_string_pretty(&all_titles()).unwrap_or_default()
}

/// `GET /api/titles`
pub async fn titles(_req: Request) -> Json<Vec<TitleJson>> {
    Json(all_titles())
}

/// `GET /api/titles/:id`
pub async fn title(req: Request) -> Result<Json<TitleJson>> {
    let id: u32 = req.param_as("id")?;
    catalog::title(id)
        .map(|t| Json(TitleJson::from(t)))
        .ok_or_else(|| Error::not_found("titre inconnu"))
}

/// `GET /api/picks?platform=netflix&kind=film&seed=42`
pub async fn picks(req: Request) -> Result<Json<Vec<TitleJson>>> {
    let platform = req.query("platform").and_then(|p| Platform::from_slug(&p));
    let kind = req.query("kind").and_then(|k| Kind::from_slug(&k));
    let seed = match req.query("seed") {
        Some(s) => s
            .parse::<u64>()
            .map_err(|_| Error::bad_request("seed doit être un nombre"))?,
        None => today_seed(),
    };
    Ok(Json(
        catalog::picks(seed, platform, kind, PICKS)
            .into_iter()
            .map(TitleJson::from)
            .collect(),
    ))
}

/// Le corps de `POST /api/recommendations`.
#[data]
#[derive(Default)]
pub struct RecommendationQuery {
    #[serde(default)]
    liked: Vec<u32>,
    #[serde(default)]
    disliked: Vec<u32>,
    #[serde(default)]
    platforms: Vec<String>,
    #[serde(default)]
    genres: Vec<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

/// Une suggestion, telle que l'API l'envoie.
#[data]
pub struct SuggestionJson {
    title: TitleJson,
    score: f32,
    reason: String,
}

/// `POST /api/recommendations`
pub async fn recommendations(req: Request) -> Result<Json<Vec<SuggestionJson>>> {
    let q: RecommendationQuery = req.json().await?;
    let profile = Profile {
        liked: q.liked,
        disliked: q.disliked,
        platforms: q
            .platforms
            .iter()
            .filter_map(|p| Platform::from_slug(p))
            .collect(),
        genres: q
            .genres
            .iter()
            .filter_map(|g| Genre::from_slug(g))
            .collect(),
        kind: q.kind.as_deref().and_then(Kind::from_slug),
    };
    let limit = q.limit.unwrap_or(6).clamp(1, 20);
    Ok(Json(
        catalog::recommend(&profile, limit)
            .into_iter()
            .map(|s| SuggestionJson {
                title: TitleJson::from(s.title),
                score: (s.score * 100.0).round() / 100.0,
                reason: s.reason.text(),
            })
            .collect(),
    ))
}

/// Les votes d'un titre, tels que l'API les envoie.
#[data]
pub struct StatJson {
    title: u32,
    likes: u32,
    dislikes: u32,
}

/// `GET /api/votes/stats`
pub async fn stats(req: Request) -> Result<Json<Vec<StatJson>>> {
    let mut rows = TitleStat::all(req.db()).await?;
    rows.sort_by_key(|r| r.title);
    Ok(Json(
        rows.into_iter()
            .map(|r| StatJson {
                title: r.title as u32,
                likes: r.likes.max(0) as u32,
                dislikes: r.dislikes.max(0) as u32,
            })
            .collect(),
    ))
}

/// Le corps de `POST /api/votes` : le vote (`liked`, `null` pour le retirer) et
/// celui qu'il remplace (`previous`).
#[data]
pub struct VoteBody {
    title: u32,
    #[serde(default)]
    liked: Option<bool>,
    #[serde(default)]
    previous: Option<bool>,
}

/// `POST /api/votes`
pub async fn vote(req: Request) -> Result<(StatusCode, Json<StatJson>)> {
    let body: VoteBody = req.json().await?;
    if catalog::title(body.title).is_none() {
        return Err(Error::not_found("titre inconnu"));
    }
    let db = req.db();
    let id = i64::from(body.title);
    let mut row = TitleStat::find(db, id).await?.unwrap_or(TitleStat {
        title: id,
        likes: 0,
        dislikes: 0,
    });
    match body.previous {
        Some(true) => row.likes = (row.likes - 1).max(0),
        Some(false) => row.dislikes = (row.dislikes - 1).max(0),
        None => {}
    }
    match body.liked {
        Some(true) => row.likes += 1,
        Some(false) => row.dislikes += 1,
        None => {}
    }
    row.upsert(db).await?;
    Ok((
        StatusCode::CREATED,
        Json(StatJson {
            title: body.title,
            likes: row.likes as u32,
            dislikes: row.dislikes as u32,
        }),
    ))
}

/// Le corps de `POST /api/proposals`.
#[data]
pub struct ProposalBody {
    title: String,
    #[serde(default)]
    platform: String,
}

/// `POST /api/proposals`
pub async fn propose(req: Request) -> Result<(StatusCode, Json<ProposalRow>)> {
    let body: ProposalBody = req.json().await?;
    let title = body.title.trim();
    let length = title.chars().count();
    if !(2..=120).contains(&length) {
        return Err(Error::bad_request(
            "le titre doit faire de 2 à 120 caractères",
        ));
    }
    let platform = Platform::from_slug(&body.platform)
        .map(|p| p.slug())
        .unwrap_or("autre");
    let mut row = ProposalRow {
        id: 0,
        title: title.to_owned(),
        platform: platform.to_owned(),
    };
    row.insert(req.db()).await?;
    Ok((StatusCode::CREATED, Json(row)))
}
