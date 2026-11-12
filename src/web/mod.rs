//! L'application : les routes, la PWA, et la configuration commune aux pages.

pub mod api;
pub mod export;
pub mod pages;
pub mod source;
pub mod tmdb;

use std::sync::RwLock;

use paquets::prelude::*;

/// Le préfixe des liens (`""`, ou `/ReelPick` sur GitHub Pages).
static BASE: RwLock<String> = RwLock::new(String::new());
/// L'adresse publique du site, pour les URL absolues.
static SITE_URL: RwLock<Option<String>> = RwLock::new(None);
/// `true` pendant l'export statique : pas de serveur sur le même site.
static STATIC_SITE: RwLock<bool> = RwLock::new(false);
/// L'API qu'appelle un site statique (le serveur Heroku), s'il y en a une.
static STATIC_API: RwLock<Option<String>> = RwLock::new(None);

/// Le préfixe des liens.
pub fn base() -> String {
    BASE.read().map(|b| b.clone()).unwrap_or_default()
}

/// Un lien interne, avec le préfixe : `link("/catalogue/")`.
pub fn link(path: &str) -> String {
    format!("{}{path}", base())
}

/// L'adresse absolue d'un chemin, si l'adresse publique du site est connue.
pub fn absolute(path: &str) -> Option<String> {
    let site = SITE_URL.read().ok()?.clone()?;
    Some(format!("{}{path}", site.trim_end_matches('/')))
}

/// L'adresse de l'API pour les îlots : le même site, le serveur qu'appelle un site
/// statique, ou `None` (site statique seul : les îlots lisent le fichier de titres).
pub fn api_base() -> Option<String> {
    if is_static() {
        STATIC_API.read().ok().and_then(|a| a.clone())
    } else {
        Some(base())
    }
}

/// Le fichier de titres d'un site statique.
pub fn pool_url() -> Option<String> {
    is_static().then(|| link("/data/titres.json"))
}

/// Le site est-il exporté en fichiers statiques ?
pub fn is_static() -> bool {
    STATIC_SITE.read().map(|s| *s).unwrap_or(false)
}

/// Règle le mode statique (export pour GitHub Pages).
pub fn set_static(base_path: &str, site_url: Option<String>, api: Option<String>) {
    if let Ok(mut a) = STATIC_API.write() {
        *a = api.map(|a| a.trim_end_matches('/').to_owned());
    }
    if let Ok(mut b) = BASE.write() {
        *b = base_path.trim_end_matches('/').to_owned();
    }
    if let Ok(mut s) = SITE_URL.write() {
        *s = site_url;
    }
    if let Ok(mut s) = STATIC_SITE.write() {
        *s = true;
    }
}

/// L'adresse publique du site (Heroku la donne par `SITE_URL`).
fn configure_site_url() {
    if let Ok(url) = std::env::var("SITE_URL")
        && !url.is_empty()
        && let Ok(mut s) = SITE_URL.write()
        && s.is_none()
    {
        *s = Some(url);
    }
}

/// La couleur du thème (barre d'état, PWA).
pub const THEME_COLOR: &str = "#0b0b14";

/// Le manifeste de l'application installable.
fn pwa() -> Pwa {
    Pwa::new("ReelPick")
        .short_name("ReelPick")
        .description("5 films et séries au hasard sur Netflix, Prime Video, Disney+ et HBO Max, vos votes et des suggestions selon vos goûts.")
        .lang("fr")
        .start_url("/")
        .scope("/")
        .display("standalone")
        .orientation("portrait")
        .theme_color(THEME_COLOR)
        .background_color(THEME_COLOR)
        .categories(["entertainment", "lifestyle"])
        .icon("/icons/icon-192.png", "192x192", "image/png")
        .icon("/icons/icon-512.png", "512x512", "image/png")
        .maskable_icon("/icons/icon-maskable-512.png", "512x512", "image/png")
}

/// L'application complète.
pub fn app() -> App {
    configure_site_url();
    let mut app = App::new();
    // Un site sur une autre origine (GitHub Pages) peut appeler l'API :
    // CORS_ORIGINS=https://maxlestage.github.io (plusieurs : séparées par des virgules).
    let origins: Vec<String> = std::env::var("CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|o| o.trim().trim_end_matches('/').to_owned())
        .filter(|o| o.starts_with("https://") || o.starts_with("http://"))
        .collect();
    if !origins.is_empty() {
        use paquets::securite::prelude::{Cors, Csrf};
        let mut csrf = Csrf::new();
        for origin in &origins {
            csrf = csrf.trusted_origin(origin);
        }
        app.security(
            paquets::shield()
                .guard(
                    Cors::new()
                        .allow_origins(origins.iter().map(String::as_str))
                        .allow_headers(["content-type"]),
                )
                .guard(csrf)
                .build(),
        );
    }
    app.model::<api::VoteCount>();
    app.model::<api::ProposalRow>();

    app.get("/", pages::home);
    app.get("/catalogue/", pages::catalogue);
    app.get("/application-ios/", pages::ios);
    app.get("/a-propos/", pages::about);
    app.get("/mentions-legales/", pages::legal);
    app.get("/confidentialite/", pages::privacy);
    app.get("/accessibilite/", pages::accessibility);
    app.get("/sitemap.xml", pages::sitemap);
    app.get("/robots.txt", pages::robots);

    app.get("/api/status", api::status);
    app.get("/api/titles", api::titles);
    app.get("/api/titles/:key", api::title);
    app.get("/api/picks", api::picks);
    app.get("/api/catalogue", api::catalogue);
    app.post("/api/recommendations", api::recommendations);
    app.get("/api/votes/stats", api::stats);
    app.post("/api/votes", api::vote);
    app.post("/api/proposals", api::propose);

    app.health("/health");
    app.pwa(pwa());
    app.fallback(pages::not_found);
    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pages_answer() {
        let client = app().test().await;
        for path in [
            "/",
            "/catalogue/",
            "/application-ios/",
            "/a-propos/",
            "/mentions-legales/",
            "/confidentialite/",
            "/accessibilite/",
            "/sitemap.xml",
            "/robots.txt",
            "/manifest.webmanifest",
            "/sw.js",
            "/health",
        ] {
            assert_eq!(client.get(path).await.status(), 200, "GET {path}");
        }
        let home = client.get("/").await.text();
        assert!(home.contains("La sélection du jour"));
        assert!(home.contains("<active-island"));
        assert!(home.contains("Suggestions pour vous"));
        assert!(home.contains("lang=\"fr\""));
        let missing = client.get("/nulle-part").await;
        assert_eq!(missing.status(), 404);
        let missing = missing.text();
        assert!(
            missing.contains("Cette page n") && missing.contains("pas à l"),
            "{missing}"
        );
    }

    /// L'application iOS embarque le catalogue : il doit rester celui du serveur.
    /// Pour le mettre à jour : `cargo run -- catalog-json > ios/ReelPick/catalog.json`.
    #[test]
    fn ios_catalog_is_up_to_date() {
        let embedded = include_str!("../../ios/ReelPick/catalog.json");
        assert_eq!(
            embedded.trim(),
            api::catalog_json().trim(),
            "ios/ReelPick/catalog.json est en retard : cargo run -- catalog-json > ios/ReelPick/catalog.json"
        );
    }

    #[tokio::test]
    async fn api_votes_and_suggestions() {
        let client = app().test().await;
        let titles = client.get("/api/titles").await;
        assert_eq!(titles.status(), 200);
        assert!(titles.text().contains("Stranger Things"));
        assert!(titles.text().contains("\"prime-video\""));
        assert_eq!(client.get("/api/titles/local:27").await.status(), 200);
        assert_eq!(client.get("/api/titles/local:999").await.status(), 404);
        assert_eq!(client.get("/api/status").await.status(), 200);

        // Sans clé TMDB, le tirage se fait dans la sélection embarquée.
        if tmdb::client().is_none() {
            let picks = client
                .get("/api/picks?platform=prime-video&kind=film&seed=7")
                .await;
            assert_eq!(picks.status(), 200);
            let picks: Vec<paquets::serde_json::Value> =
                paquets::serde_json::from_str(&picks.text()).unwrap();
            assert_eq!(picks.len(), 5);
            assert!(picks.iter().all(|t| t["platforms"][0] == "prime-video"));
            let page = client
                .get("/api/catalogue?platform=hbo-max&kind=serie")
                .await
                .text();
            assert!(page.contains("Succession"), "{page}");
        }

        let res = client
            .post("/api/votes")
            .json(&json!({ "title": "local:25", "liked": true }))
            .await;
        assert_eq!(res.status(), 201, "{}", res.text());
        client
            .post("/api/votes")
            .json(&json!({ "title": "local:25", "previous": true, "liked": false }))
            .await;
        client
            .post("/api/votes")
            .json(&json!({ "title": "movie:603", "liked": true }))
            .await;
        let stats = client.get("/api/votes/stats").await.text();
        assert!(
            stats.contains(r#"{"title":"local:25","likes":0,"dislikes":1}"#),
            "{stats}"
        );
        assert!(
            stats.contains(r#"{"title":"movie:603","likes":1,"dislikes":0}"#),
            "{stats}"
        );
        assert_eq!(
            client
                .post("/api/votes")
                .json(&json!({ "title": "local:999", "liked": true }))
                .await
                .status(),
            404
        );

        if tmdb::client().is_none() {
            let liked = |key: &str| {
                paquets::serde_json::to_value(reelpick::catalog::builtin_title(key).unwrap().seen())
                    .unwrap()
            };
            let rec = client
                .post("/api/recommendations")
                .json(&json!({ "liked": [liked("local:25"), liked("local:26")], "limit": 3 }))
                .await;
            assert_eq!(rec.status(), 200);
            let rec = rec.text();
            assert!(rec.contains("Parce que vous avez aimé"), "{rec}");
        }

        let res = client
            .post("/api/proposals")
            .json(&json!({ "title": "Severance", "platform": "prime-video" }))
            .await;
        assert_eq!(res.status(), 201);
        let res = client
            .post("/api/proposals")
            .json(&json!({ "title": " ", "platform": "netflix" }))
            .await;
        assert_eq!(res.status(), 400);
    }
}
