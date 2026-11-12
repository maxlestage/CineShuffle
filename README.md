<div align="center">

<img src="public/icons/icon-192.png" width="96" height="96" alt="">

# ReelPick

**Ce soir, on regarde quoi ?**

5 films et séries tirés au sort sur **Netflix**, **HBO Max** et **Disney+**, des votes pour ce que vous avez vu, et des suggestions qui s'affinent selon vos goûts.

Site en **Rust** avec [Paquets](https://github.com/maxlestage/Paquets) · PWA installable · application **iOS** en Swift

</div>

---

## Ce que fait ReelPick

| | |
|---|---|
| 🎲 **Tirage au sort** | 5 titres parmi 60 films et séries choisis, filtrables par plateforme et par format. La sélection du jour change chaque nuit ; « Mélanger » en tire une autre. |
| 👍 **Votes** | « Aimé » ou « Pas aimé » sur chaque titre vu. Les votes restent sur l'appareil ; sur le site hébergé, ils sont aussi comptés (anonymement) pour la communauté. |
| ✨ **Suggestions** | Dans le pied de page : vos plateformes, vos genres, films ou séries, et les titres les plus proches de vos goûts (genres en commun, format, plateforme), avec la raison de chaque suggestion. |
| 💌 **Proposer un titre** | Un formulaire dans le pied de page, enregistré par l'API. |
| 📱 **Mobile d'abord** | Conçu pour le téléphone, de 320 px aux grands écrans, **sans aucun défilement horizontal** (vérifié à 320, 360, 390, 768 et 1280 px). |
| 📲 **PWA** | Installable sur l'écran d'accueil, utilisable hors ligne (manifeste, service worker, icônes). |
| 🎞️ **2D et animations** | Affiches dessinées en CSS (dégradés, anneaux qui tournent, emoji qui flottent), éventail de cartes, pellicule qui défile, cartes « distribuées » à chaque tirage, rebond des votes. Coupées si le système demande moins de mouvement. |
| 🍎 **App iOS** | SwiftUI, la même sélection du jour que le site (même générateur), votes, suggestions, retour haptique. Fonctionne hors ligne. |

## Comment c'est construit

```text
src/catalog.rs        le catalogue, le tirage (splitmix64) et le moteur de suggestions
src/lib.rs            les îlots (Rust → WebAssembly) : `picker` (tirage, votes) et `taste` (pied de page)
src/main.rs           le serveur (et `export`, `catalog-json`)
src/web/pages.rs      les pages : accueil, catalogue, app iOS, à propos, mentions, confidentialité…
src/web/api.rs        l'API JSON (site et app iOS), avec la base de données
src/web/export.rs     le site en fichiers statiques pour GitHub Pages
src/web/style.css     le style, mobile d'abord, et les animations
public/               icônes, favicon, image Open Graph
ios/                  l'application iOS (SwiftUI, projet XcodeGen)
.github/workflows/    CI + fusion automatique, Heroku, GitHub Pages, iOS/TestFlight
RustConfig, heroku-build.sh, Procfile, app.json   Heroku avec le buildpack emk/rust
```

Les pages sont rendues par le serveur (SEO, contenu visible sans JavaScript) ; seules les parties interactives sont des **îlots** compilés en WebAssembly. Le même code Rust tire les titres côté serveur et dans le navigateur.

### L'API

| Route | Rôle |
|---|---|
| `GET /api/titles` | le catalogue |
| `GET /api/titles/:id` | un titre |
| `GET /api/picks?platform=netflix&kind=film&seed=42` | 5 titres au hasard (la sélection du jour sans `seed`) |
| `POST /api/recommendations` | `{"liked":[25,26],"disliked":[],"platforms":["netflix"],"genres":["drame"],"kind":"serie","limit":6}` |
| `GET /api/votes/stats` | les votes de la communauté, par titre |
| `POST /api/votes` | `{"title":25,"liked":true}` ; changer d'avis : `{"title":25,"previous":true,"liked":false}` |
| `POST /api/proposals` | `{"title":"Severance","platform":"netflix"}` |
| `GET /health` | l'état du service |

## Tout faire depuis un téléphone

Aucun ordinateur n'est nécessaire : GitHub Actions compile, teste, fusionne et déploie.

### 1. Fusion automatique dans `master`

Chaque push sur une branche lance la CI (format, clippy du serveur et des îlots, tests, compilation WebAssembly, export statique). **Quand tout est vert, la branche est fusionnée dans `master`** (sauf les branches `wip/…`), puis Heroku, GitHub Pages et l'app iOS sont déployés.

> Si les dépôts de Paquets sont privés : ajoutez un secret **`REPOS_TOKEN`** (un jeton GitHub en lecture seule sur ces dépôts) dans *Settings → Secrets and variables → Actions*.

### 2. GitHub Pages

Une seule fois, dans l'app GitHub ou sur github.com : **Settings → Pages → Source : « GitHub Actions »**. Le workflow *GitHub Pages* publie ensuite le site à chaque fusion, et chaque nuit (nouvelle sélection du jour) : `https://maxlestage.github.io/ReelPick/`.

Sur GitHub Pages il n'y a pas de serveur : tout fonctionne, et les votes restent sur l'appareil.

### 3. Heroku (buildpack `emk/rust`)

Dans *Settings → Secrets and variables → Actions* :

| Où | Nom | Valeur |
|---|---|---|
| Secret | `HEROKU_API_KEY` | Heroku → *Account settings* → *API Key* → *Reveal* |
| Variable | `HEROKU_APP_NAME` | le nom de l'app (créée automatiquement si elle n'existe pas) |
| Variable (facultatif) | `HEROKU_REGION` | `eu` (par défaut) ou `us` |
| Variable (facultatif) | `HEROKU_POSTGRES_PLAN` | par exemple `essential-0`, pour garder les votes dans PostgreSQL |

Puis *Actions → Heroku → Run workflow* (ou un simple push). Le workflow crée l'app sur la pile `heroku-26`, installe le buildpack **`emk/rust`**, envoie le code et suit la compilation : `RustConfig` installe Rust, `heroku-build.sh` compile le serveur et les îlots, le `Procfile` démarre `target/release/reelpick`.

Si le dépôt est public, le bouton marche aussi :

[![Deploy to Heroku](https://www.herokucdn.com/deploy/button.svg)](https://www.heroku.com/deploy?template=https://github.com/maxlestage/ReelPick)

### 4. L'app iOS sur TestFlight

Le workflow *iOS* génère le projet avec XcodeGen, compile et lance les tests sur un simulateur, sur un Mac de GitHub. Pour l'installer sur votre iPhone avec TestFlight :

1. Sur [developer.apple.com](https://developer.apple.com/account) (compte Apple Developer), notez votre **Team ID** (*Membership*).
2. Sur [App Store Connect](https://appstoreconnect.apple.com) : créez l'app (identifiant `com.maxlestage.reelpick`), puis *Utilisateurs et accès → Intégrations → Clés d'API* : créez une clé (rôle *Admin*), téléchargez le `.p8`, notez son **Key ID** et l'**Issuer ID**.
3. Dans le dépôt, ajoutez les secrets `APPLE_TEAM_ID`, `ASC_KEY_ID`, `ASC_ISSUER_ID` et `ASC_KEY_P8` (le contenu du `.p8`), et la variable `REELPICK_API_URL` (l'adresse de l'app Heroku).
4. *Actions → iOS → Run workflow* : la build arrive dans TestFlight quelques minutes plus tard.

## Sur un ordinateur (facultatif)

```sh
cargo install --git https://github.com/maxlestage/Paquets paquets --locked
paquets dev                                   # http://localhost:3000, îlots compris
cargo test                                    # catalogue, tirage, suggestions, pages, API
BASE_PATH=/ReelPick cargo run -- export dist  # le site statique
cargo run -- catalog-json > ios/ReelPick/catalog.json   # après un changement du catalogue
cd ios && xcodegen generate && open ReelPick.xcodeproj  # l'app iOS (macOS)
```

## Mentions

Netflix, HBO Max et Disney+ sont des marques de leurs propriétaires respectifs ; ReelPick n'est affilié à aucune d'elles. Les disponibilités sont indicatives (France) ; les résumés et les affiches illustrées sont originaux.
