//! L'application : les routes, la PWA, et la configuration commune aux pages.

pub mod api;
pub mod export;
pub mod pages;

use std::sync::RwLock;

use paquets::prelude::*;

/// Le préfixe des liens (`""`, ou `/ReelPick` sur GitHub Pages).
static BASE: RwLock<String> = RwLock::new(String::new());
/// L'adresse publique du site, pour les URL absolues.
static SITE_URL: RwLock<Option<String>> = RwLock::new(None);
/// `true` pendant l'export statique : pas d'API, les votes restent dans le navigateur.
static STATIC_SITE: RwLock<bool> = RwLock::new(false);

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

/// L'adresse de l'API pour les îlots : `Some("")` (le même site) ou `None` (site statique).
pub fn api_base() -> Option<String> {
    if is_static() { None } else { Some(base()) }
}

/// Le site est-il exporté en fichiers statiques ?
pub fn is_static() -> bool {
    STATIC_SITE.read().map(|s| *s).unwrap_or(false)
}

/// Règle le mode statique (export pour GitHub Pages).
pub fn set_static(base_path: &str, site_url: Option<String>) {
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
        .description("5 films et séries au hasard sur Netflix, HBO Max et Disney+, vos votes et des suggestions selon vos goûts.")
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
    app.model::<api::TitleStat>();
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

    app.get("/api/titles", api::titles);
    app.get("/api/titles/:id", api::title);
    app.get("/api/picks", api::picks);
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
        assert_eq!(client.get("/api/titles/27").await.status(), 200);
        assert_eq!(client.get("/api/titles/999").await.status(), 404);

        let picks = client
            .get("/api/picks?platform=disney-plus&kind=film&seed=7")
            .await;
        assert_eq!(picks.status(), 200);
        let picks: Vec<paquets::serde_json::Value> =
            paquets::serde_json::from_str(&picks.text()).unwrap();
        assert_eq!(picks.len(), 5);
        assert!(picks.iter().all(|t| t["platform"] == "disney-plus"));

        let res = client
            .post("/api/votes")
            .json(&json!({ "title": 25, "liked": true }))
            .await;
        assert_eq!(res.status(), 201, "{}", res.text());
        client
            .post("/api/votes")
            .json(&json!({ "title": 25, "previous": true, "liked": false }))
            .await;
        client
            .post("/api/votes")
            .json(&json!({ "title": 26, "liked": true }))
            .await;
        let stats = client.get("/api/votes/stats").await.text();
        assert!(
            stats.contains(r#"{"title":25,"likes":0,"dislikes":1}"#),
            "{stats}"
        );
        assert!(
            stats.contains(r#"{"title":26,"likes":1,"dislikes":0}"#),
            "{stats}"
        );
        assert_eq!(
            client
                .post("/api/votes")
                .json(&json!({ "title": 999, "liked": true }))
                .await
                .status(),
            404
        );

        let rec = client
            .post("/api/recommendations")
            .json(&json!({ "liked": [25, 26], "disliked": [], "limit": 3 }))
            .await;
        assert_eq!(rec.status(), 200);
        let rec = rec.text();
        assert!(rec.contains("Parce que vous avez aimé"), "{rec}");

        let res = client
            .post("/api/proposals")
            .json(&json!({ "title": "Severance", "platform": "netflix" }))
            .await;
        assert_eq!(res.status(), 201);
        let res = client
            .post("/api/proposals")
            .json(&json!({ "title": " ", "platform": "netflix" }))
            .await;
        assert_eq!(res.status(), 400);
    }
}
