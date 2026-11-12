//! D'où viennent les titres : le catalogue complet de Wikidata (avec Wikipédia pour les
//! fiches), l'instantané d'un export statique, ou la sélection embarquée. Si Wikidata
//! n'est pas encore chargé ou ne répond pas, CineShuffle continue avec la sélection
//! embarquée.

use std::sync::{Arc, RwLock};

use cineshuffle::catalog::{Kind, Platform, Profile, Suggestion, Title, builtin, builtin_title};
use cineshuffle::{local_picks, local_suggestions};

use super::wiki;

/// Les titres d'un export statique (instantané de Wikidata, ou la sélection embarquée).
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

/// Le catalogue de Wikidata, sauf pendant un export statique.
fn live() -> Option<Arc<wiki::Catalogue>> {
    if pool().is_some() {
        None
    } else {
        wiki::current().filter(|c| !c.is_empty())
    }
}

/// Le nom de la source : `wikidata` ou `selection`.
pub fn name() -> &'static str {
    if live().is_some() {
        "wikidata"
    } else {
        "selection"
    }
}

/// Le nombre de titres connus.
pub fn size() -> usize {
    live().map_or_else(|| offline().len(), |c| c.len())
}

/// 5 titres tirés au sort.
pub async fn picks(seed: u64, platform: Option<Platform>, kind: Option<Kind>) -> Vec<Title> {
    if let Some(catalogue) = live() {
        let found = wiki::draw(&catalogue, seed, platform, kind).await;
        if !found.is_empty() {
            return found;
        }
    }
    let pool = pool();
    local_picks(pool.as_deref(), seed, platform, kind)
}

/// Les suggestions d'un profil.
pub async fn recommend(profile: &Profile, n: usize) -> Vec<Suggestion> {
    if let Some(catalogue) = live() {
        return wiki::suggest(&catalogue, profile, n).await;
    }
    let pool = pool();
    let mut found = local_suggestions(pool.as_deref(), profile);
    found.truncate(n);
    found
}

/// Un titre par sa clé, avec sa fiche complète.
pub async fn title(key: &str) -> Option<Title> {
    if let Some(t) = builtin_title(key) {
        return Some(t.clone());
    }
    if let Some(t) = pool().and_then(|p| p.into_iter().find(|t| t.key == key)) {
        return Some(t);
    }
    let catalogue = live()?;
    let t = catalogue.get(key)?.clone();
    Some(wiki::details(&catalogue, &t, true).await)
}

/// Une page du catalogue d'une plateforme et le nombre de pages (`None` sans Wikidata).
pub async fn browse(platform: Platform, kind: Kind, page: u32) -> Option<(Vec<Title>, u32)> {
    let catalogue = live()?;
    Some(wiki::browse(&catalogue, platform, kind, page).await)
}

/// Les titres connus sans Wikidata : ceux de l'export statique, ou la sélection embarquée.
pub fn offline() -> Vec<Title> {
    pool().unwrap_or_else(|| builtin().to_vec())
}

/// Un instantané pour un site statique : les titres les plus connus de chaque
/// plateforme (films et séries), avec résumé et affiche, ou la sélection embarquée si
/// Wikidata ne répond pas.
pub async fn snapshot(per_list: usize) -> Vec<Title> {
    // `CINESHUFFLE_OFFLINE` : aucun appel réseau (la CI vérifie l'export ainsi).
    if std::env::var_os("CINESHUFFLE_OFFLINE").is_some() {
        return builtin().to_vec();
    }
    let catalogue = match wiki::current().filter(|c| !c.is_empty()) {
        Some(c) => c,
        None => match wiki::fetch().await {
            Ok(c) => wiki::install(c),
            Err(e) => {
                eprintln!("{e} : instantané de la sélection embarquée");
                return builtin().to_vec();
            }
        },
    };
    let titles = wiki::snapshot(&catalogue, per_list).await;
    if titles.is_empty() {
        builtin().to_vec()
    } else {
        titles
    }
}
