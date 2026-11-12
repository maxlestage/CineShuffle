//! `cineshuffle export <dossier>` : le site en fichiers statiques, pour GitHub Pages.
//!
//! Chaque page vient du serveur lui-même, par le client de test en mémoire de Paquets
//! (`app.test()`) : le site statique et le site hébergé sont identiques. Sans serveur,
//! les îlots gardent les votes et les titres proposés dans le navigateur.

use std::io;
use std::path::{Path, PathBuf};

use super::pages::{PAGES, not_found_page};
use super::{app, set_static, source};

/// Le nombre de pages de TMDB (20 titres) gardées par plateforme et par format dans
/// l'instantané d'un site statique.
const SNAPSHOT_PAGES: u32 = 5;

/// Écrit le site dans `out`, liens derrière `base` (`""` ou `/CineShuffle`), adresses
/// absolues sur `site_url`. Renvoie le nombre de fichiers écrits.
pub async fn export(out: &Path, base: &str, site_url: Option<String>) -> io::Result<usize> {
    let base = base.trim_end_matches('/').to_owned();
    let site_url = site_url
        .or_else(|| (!base.is_empty()).then(|| format!("https://maxlestage.github.io{base}")));
    // Le serveur qu'appelle le site statique (CINESHUFFLE_API_URL, par exemple l'app Heroku) :
    // tout le catalogue. Sans lui, les îlots tirent dans l'instantané `data/titres.json`.
    let api = std::env::var("CINESHUFFLE_API_URL")
        .ok()
        .map(|u| u.trim().trim_end_matches('/').to_owned())
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"));
    // L'instantané d'abord : TMDB est ensuite coupé pendant l'export.
    let pool = source::snapshot(SNAPSHOT_PAGES).await;
    println!("{} titres dans l'instantané", pool.len());
    source::set_pool(pool.clone());
    set_static(&base, site_url, api);
    if !Path::new("pkg").is_dir() {
        eprintln!("attention : pkg/ manque, les îlots ne seront pas interactifs (voir README.md)");
    }
    let out = std::path::absolute(out)?;
    let mut app = app();
    // Des fichiers statiques n'ont pas de serveur à protéger, et l'export demande toutes
    // les pages d'un coup (la limite de débit l'arrêterait).
    app.security(None);
    let client = app.test().await;

    let mut written = 0;
    let mut write = |path: &str, bytes: &[u8]| -> io::Result<()> {
        let file = out.join(path.trim_start_matches('/'));
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, bytes)?;
        written += 1;
        Ok(())
    };

    for page in PAGES {
        let res = client.get(page).await;
        if res.status() != 200 {
            return Err(io::Error::other(format!("GET {page} : {}", res.status())));
        }
        write(
            &format!("{page}index.html"),
            with_base(&res.text(), &base).as_bytes(),
        )?;
    }
    let missing = not_found_page().await.render();
    write("404.html", with_base(&missing, &base).as_bytes())?;
    write(
        "/data/titres.json",
        paquets::serde_json::to_string(&pool)
            .map_err(io::Error::other)?
            .as_bytes(),
    )?;
    for file in ["/manifest.webmanifest", "/sw.js"] {
        let res = client.get(file).await;
        if res.status() != 200 {
            return Err(io::Error::other(format!("GET {file} : {}", res.status())));
        }
        write(file, with_base_everywhere(&res.text(), &base).as_bytes())?;
    }
    for file in ["/sitemap.xml", "/robots.txt"] {
        let body = client.get(file).await.text();
        // Pas d'API sur un hébergement statique.
        write(file, body.replace("Disallow: /api/\n", "").as_bytes())?;
    }
    for folder in ["public", "pkg"] {
        let root = PathBuf::from(folder);
        let prefix = if folder == "pkg" { "pkg/" } else { "" };
        for file in files(&root) {
            let relative = file.strip_prefix(&root).map_err(io::Error::other)?;
            let url = format!("{prefix}{}", relative.to_string_lossy().replace('\\', "/"));
            // Les fichiers de wasm-bindgen inutiles au navigateur.
            if url.ends_with(".gitignore") || url.ends_with("package.json") {
                continue;
            }
            write(&url, &std::fs::read(&file)?)?;
        }
    }
    // Le catalogue, lisible sans serveur (par l'application iOS, par exemple).
    write("/api/titles.json", super::api::catalog_json().as_bytes())?;
    // Les fichiers sont servis tels quels : pas de Jekyll sur GitHub Pages.
    write(".nojekyll", b"")?;
    Ok(written)
}

/// Les adresses que Paquets écrit lui-même (le chargeur des îlots, la PWA) ne
/// connaissent pas le préfixe : elles le reçoivent ici. Les liens du site l'ont déjà.
pub fn with_base(html: &str, base: &str) -> String {
    if base.is_empty() {
        return html.to_owned();
    }
    html.replace("\"/pkg/", &format!("\"{base}/pkg/"))
        .replace(
            "\"/manifest.webmanifest\"",
            &format!("\"{base}/manifest.webmanifest\""),
        )
        .replace("\"/sw.js\"", &format!("\"{base}/sw.js\""))
        .replace("'/sw.js'", &format!("'{base}/sw.js'"))
}

/// Toutes les adresses absolues (`"/…"`) d'un fichier JSON ou JavaScript, derrière `base`.
pub fn with_base_everywhere(text: &str, base: &str) -> String {
    if base.is_empty() {
        return text.to_owned();
    }
    text.replace("\"/", &format!("\"{base}/"))
}

/// Tous les fichiers sous `dir`.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}
