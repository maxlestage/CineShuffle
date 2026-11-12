//! Le serveur de ReelPick : les pages, l'API JSON (utilisée par le site et
//! l'application iOS) et la base de données, avec la sécurité de Paquets.
//!
//!     cargo run                      # http://localhost:3000 ($PORT sur Heroku)
//!     cargo run -- export dist       # le site en fichiers statiques (GitHub Pages)
//!     cargo run -- catalog-json      # le catalogue en JSON (pour l'application iOS)
//!
//! `BASE_PATH` (par exemple `/ReelPick`) préfixe les liens de l'export, `SITE_URL`
//! donne les adresses absolues (balises canoniques, Open Graph, sitemap).

mod web;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("export") => {
            let dir = args.next().unwrap_or_else(|| "dist".into());
            let base = std::env::var("BASE_PATH").unwrap_or_default();
            let site_url = std::env::var("SITE_URL").ok().filter(|u| !u.is_empty());
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let files = runtime.block_on(web::export::export(
                std::path::Path::new(&dir),
                &base,
                site_url,
            ))?;
            println!("🎬 {files} fichiers écrits dans {dir}");
            Ok(())
        }
        Some("catalog-json") => {
            // Le catalogue en JSON, embarqué dans l'application iOS (ios/ReelPick/catalog.json).
            println!("{}", web::api::catalog_json());
            Ok(())
        }
        Some(other) => {
            eprintln!("commande inconnue {other:?} : lancez sans argument, ou `export <dossier>`");
            std::process::exit(2);
        }
        None => web::app().run(),
    }
}
