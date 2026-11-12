//! D'où viennent les titres : TMDB (tout le catalogue) quand une clé est configurée,
//! le fichier de titres d'un export statique, ou la sélection embarquée. Si TMDB ne
//! répond pas, CineShuffle continue avec la sélection embarquée.

use std::sync::RwLock;

use cineshuffle::catalog::{
    Kind, PICKS, Platform, Profile, Suggestion, Title, builtin, builtin_title,
};
use cineshuffle::{local_picks, local_suggestions};

use super::tmdb;

/// Les titres d'un export statique (instantané de TMDB, ou la sélection embarquée).
static POOL: RwLock<Option<Vec<Title>>> = RwLock::new(None);

/// Règle les titres de l'export statique.
pub fn set_pool(titles: Vec<Title>) {
    if let Ok(mut pool) = POOL.write() {
        *pool = Some(titles);
    }
}

fn pool() -> Option<Vec<Title>> {
    POOL.read().ok().and_then(|p| p.clone())
}

/// Le nom de la source : `tmdb` ou `selection`.
pub fn name() -> &'static str {
    if tmdb::client().is_some() && pool().is_none() {
        "tmdb"
    } else {
        "selection"
    }
}

/// TMDB, sauf pendant un export statique.
fn live() -> Option<std::sync::Arc<tmdb::Tmdb>> {
    if pool().is_some() {
        None
    } else {
        tmdb::client()
    }
}

/// 5 titres tirés au sort.
pub async fn picks(seed: u64, platform: Option<Platform>, kind: Option<Kind>) -> Vec<Title> {
    if let Some(tmdb) = live() {
        match tmdb.picks(seed, platform, kind, PICKS).await {
            Ok(found) if !found.is_empty() => return found,
            Ok(_) => {}
            Err(e) => eprintln!("{e} : tirage dans la sélection embarquée"),
        }
    }
    let pool = pool();
    local_picks(pool.as_deref(), seed, platform, kind)
}

/// Les suggestions d'un profil.
pub async fn recommend(profile: &Profile, n: usize) -> Vec<Suggestion> {
    if let Some(tmdb) = live() {
        match tmdb.recommend(profile, n).await {
            Ok(found) => return found,
            Err(e) => eprintln!("{e} : suggestions dans la sélection embarquée"),
        }
    }
    let pool = pool();
    let mut found = local_suggestions(pool.as_deref(), profile);
    found.truncate(n);
    found
}

/// Un titre par sa clé.
pub async fn title(key: &str) -> Option<Title> {
    if let Some(t) = builtin_title(key) {
        return Some(t.clone());
    }
    if let Some(t) = pool().and_then(|p| p.into_iter().find(|t| t.key == key)) {
        return Some(t);
    }
    live()?.title(key).await.ok()
}

/// Une page du catalogue d'une plateforme, et le nombre de pages (`None` sans TMDB).
pub async fn browse(platform: Platform, kind: Kind, page: u32) -> Option<(Vec<Title>, u32)> {
    live()?.browse(platform, kind, page).await.ok()
}

/// Les titres connus sans TMDB : ceux de l'export statique, ou la sélection embarquée.
pub fn offline() -> Vec<Title> {
    pool().unwrap_or_else(|| builtin().to_vec())
}

/// Un instantané du catalogue pour un site statique : les titres les plus populaires
/// de chaque plateforme (films et séries) si TMDB est configuré, la sélection
/// embarquée sinon.
pub async fn snapshot(pages: u32) -> Vec<Title> {
    let Some(tmdb) = tmdb::client() else {
        return builtin().to_vec();
    };
    let mut all: Vec<Title> = Vec::new();
    for platform in Platform::ALL {
        for kind in [Kind::Film, Kind::Serie] {
            for page in 1..=pages {
                match tmdb.browse(platform, kind, page).await {
                    Ok((titles, total)) => {
                        for t in titles {
                            match all.iter_mut().find(|known| known.key == t.key) {
                                Some(known) => {
                                    if !known.platforms.contains(&platform) {
                                        known.platforms.push(platform);
                                    }
                                }
                                None => all.push(t),
                            }
                        }
                        if page >= total {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        break;
                    }
                }
            }
        }
    }
    if all.is_empty() {
        builtin().to_vec()
    } else {
        all
    }
}
