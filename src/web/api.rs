//! L'API JSON, utilisée par les îlots du site et par l'application iOS.
//!
//! | Route | Rôle |
//! |---|---|
//! | `GET /api/status` | la source des titres (`wikidata` ou `selection`), leur nombre et les plateformes |
//! | `GET /api/picks?platform=&kind=&seed=` | 5 titres au hasard dans tout le catalogue (la sélection du jour sans `seed`) |
//! | `GET /api/titles/:key` | un titre complet (`wd:Q83495` ou `local:12`) |
//! | `GET /api/catalogue?platform=&kind=&page=` | une page du catalogue d'une plateforme |
//! | `POST /api/recommendations` | des suggestions selon des votes et des préférences |
//! | `GET /api/titles` | la sélection embarquée (le secours hors ligne) |
//! | `GET /api/votes/stats` | les votes de la communauté, par titre |
//! | `POST /api/votes` | un vote (`liked`), ou son changement (`previous`) |
//! | `POST /api/proposals` | proposer un titre |

use std::time::{SystemTime, UNIX_EPOCH};

use cineshuffle::catalog::{self, Kind, Platform, Profile, Suggestion, Title};
use paquets::prelude::*;

use super::source;

/// Les votes d'un titre.
#[model]
pub struct VoteCount {
    /// La clé du titre (`wd:Q83495` ou `local:12`).
    #[memoire(primary_key)]
    pub title: String,
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

/// La graine de la sélection du jour.
pub fn today_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    catalog::day_seed(now)
}

/// La sélection embarquée en JSON lisible (le fichier de l'application iOS).
pub fn catalog_json() -> String {
    paquets::serde_json::to_string_pretty(catalog::builtin()).unwrap_or_default()
}

/// `GET /api/status`
pub async fn status(_req: Request) -> Json<paquets::serde_json::Value> {
    Json(json!({
        "source": source::name(),
        "titles": source::size(),
        "platforms": Platform::ALL.iter().map(|p| json!({ "id": p.slug(), "name": p.label() })).collect::<Vec<_>>(),
        "attribution": "Catalogue : Wikidata (CC0). Résumés et affiches : Wikipédia (CC BY-SA, et droits de leurs auteurs).",
    }))
}

/// `GET /api/titles` : la sélection embarquée.
pub async fn titles(_req: Request) -> Json<Vec<Title>> {
    Json(catalog::builtin().to_vec())
}

/// `GET /api/titles/:key`
pub async fn title(req: Request) -> Result<Json<Title>> {
    let key = req.param("key").unwrap_or_default().to_owned();
    source::title(&key)
        .await
        .map(Json)
        .ok_or_else(|| Error::not_found("titre inconnu"))
}

/// `platform` et `kind` d'une requête.
fn filters(req: &Request) -> (Option<Platform>, Option<Kind>) {
    (
        req.query("platform").and_then(|p| Platform::from_slug(&p)),
        req.query("kind").and_then(|k| Kind::from_slug(&k)),
    )
}

/// `GET /api/picks?platform=prime-video&kind=film&seed=42`
pub async fn picks(req: Request) -> Result<Json<Vec<Title>>> {
    let (platform, kind) = filters(&req);
    let seed = match req.query("seed") {
        Some(s) => s
            .parse::<u64>()
            .map_err(|_| Error::bad_request("seed doit être un nombre"))?,
        None => today_seed(),
    };
    Ok(Json(source::picks(seed, platform, kind).await))
}

/// Une page du catalogue.
#[data]
pub struct CataloguePage {
    titles: Vec<Title>,
    page: u32,
    pages: u32,
}

/// `GET /api/catalogue?platform=netflix&kind=film&page=2`
pub async fn catalogue(req: Request) -> Result<Json<CataloguePage>> {
    let (platform, kind) = filters(&req);
    let platform = platform.unwrap_or(Platform::Netflix);
    let kind = kind.unwrap_or(Kind::Film);
    let page = req
        .query("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 500);
    match source::browse(platform, kind, page).await {
        Some((titles, pages)) => Ok(Json(CataloguePage {
            titles,
            page,
            pages,
        })),
        None => {
            let titles: Vec<Title> = source::offline()
                .into_iter()
                .filter(|t| t.platforms.contains(&platform) && t.kind == kind)
                .collect();
            Ok(Json(CataloguePage {
                titles,
                page: 1,
                pages: 1,
            }))
        }
    }
}

/// Le corps de `POST /api/recommendations` : un profil (`liked`, `disliked`,
/// `platforms`, `genres`, `kind`) et le nombre de suggestions voulues.
#[data]
pub struct RecommendationQuery {
    #[serde(flatten)]
    profile: Profile,
    #[serde(default)]
    limit: Option<usize>,
}

/// `POST /api/recommendations`
pub async fn recommendations(req: Request) -> Result<Json<Vec<Suggestion>>> {
    let q: RecommendationQuery = req.json().await?;
    let limit = q.limit.unwrap_or(4).clamp(1, 20);
    Ok(Json(source::recommend(&q.profile, limit).await))
}

/// Les votes d'un titre, tels que l'API les envoie.
#[data]
pub struct StatJson {
    title: String,
    likes: u32,
    dislikes: u32,
}

/// `GET /api/votes/stats`
pub async fn stats(req: Request) -> Result<Json<Vec<StatJson>>> {
    let mut rows = VoteCount::all(req.db()).await?;
    rows.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(Json(
        rows.into_iter()
            .map(|r| StatJson {
                title: r.title,
                likes: r.likes.max(0) as u32,
                dislikes: r.dislikes.max(0) as u32,
            })
            .collect(),
    ))
}

/// Une clé de titre valide : de la sélection embarquée, ou de Wikidata (`wd:Q…`).
fn valid_key(key: &str) -> bool {
    catalog::builtin_title(key).is_some()
        || key.strip_prefix("wd:Q").is_some_and(|n| {
            !n.is_empty() && n.len() <= 12 && n.bytes().all(|b| b.is_ascii_digit())
        })
}

/// Le corps de `POST /api/votes` : le vote (`liked`, `null` pour le retirer) et
/// celui qu'il remplace (`previous`).
#[data]
pub struct VoteBody {
    title: String,
    #[serde(default)]
    liked: Option<bool>,
    #[serde(default)]
    previous: Option<bool>,
}

/// `POST /api/votes`
pub async fn vote(req: Request) -> Result<(StatusCode, Json<StatJson>)> {
    let body: VoteBody = req.json().await?;
    if !valid_key(&body.title) {
        return Err(Error::not_found("titre inconnu"));
    }
    let db = req.db();
    let mut row = VoteCount::find(db, body.title.clone())
        .await?
        .unwrap_or(VoteCount {
            title: body.title.clone(),
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
            title: row.title,
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
