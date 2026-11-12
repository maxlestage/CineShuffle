<div align="center">

<img src="public/icons/icon-192.png" width="96" height="96" alt="">

# CineShuffle

**Ce soir, on regarde quoi ?**

5 films et séries tirés au sort dans **tout le catalogue** de **Netflix**, **Prime Video**, **Disney+** et **HBO Max**, avec affiche, résumé, acteurs et année, des votes pour ce que vous avez vu, et des suggestions qui s'affinent selon vos goûts.

Site en **Rust** avec [Paquets](https://github.com/maxlestage/Paquets) · PWA installable · application **iOS** en Swift

</div>

---

## Ce que fait CineShuffle

| | |
|---|---|
| 🎲 **Tirage au sort** | 5 titres tirés parmi **les quelque 27 000 films et séries** que Wikidata associe à Netflix, Prime Video, Disney+ et HBO Max, filtrables par plateforme et par format. Chaque plateforme a sa chance, et les titres connus sortent plus souvent. La sélection du jour change chaque nuit ; « Mélanger » en tire une autre. |
| 🖼️ **Fiches complètes** | Résumé en français (Wikipédia), affiche, acteurs principaux, année, durée ou nombre de saisons, genres et plateformes. |
| 📚 **Catalogue** | Tout le catalogue de chaque plateforme, page par page, les titres les plus connus d'abord. |
| 👍 **Votes** | « Aimé » ou « Pas aimé » sur chaque titre vu. Les votes restent sur l'appareil ; sur le site hébergé, ils sont aussi comptés (anonymement) pour la communauté. |
| ✨ **Suggestions** | Dans le pied de page : vos plateformes, vos genres, films ou séries, et les titres les plus proches de vos goûts (genres en commun, format, plateforme), avec la raison de chaque suggestion. |
| 💌 **Proposer un titre** | Un formulaire dans le pied de page, enregistré par l'API. |
| 📱 **Mobile d'abord** | Conçu pour le téléphone, de 320 px aux grands écrans, **sans aucun défilement horizontal** (vérifié à 320, 360, 390, 768 et 1280 px). |
| 📲 **PWA** | Installable sur l'écran d'accueil, utilisable hors ligne (manifeste, service worker, icônes). |
| 🎞️ **2D et animations** | Affiches dessinées en CSS (dégradés, anneaux qui tournent, emoji qui flottent), éventail de cartes, pellicule qui défile, cartes « distribuées » à chaque tirage, rebond des votes. Coupées si le système demande moins de mouvement. |
| 🍎 **App iOS** | SwiftUI : le même tirage dans tout le catalogue (par l'API du serveur), affiches, acteurs, votes, suggestions, retour haptique. Hors ligne, elle utilise la sélection embarquée. |
| 🔓 **Sans clé ni compte** | Les données viennent de Wikidata et Wikipédia : libres, gratuites, sans inscription. Si elles ne répondent pas, CineShuffle utilise sa sélection embarquée de 80 titres (20 par plateforme). |

## Comment c'est construit

```text
src/catalog.rs        le modèle des titres, la sélection embarquée, le tirage (splitmix64) et le moteur de suggestions
src/web/wiki.rs       Wikidata (catalogue, acteurs, durée) et Wikipédia (résumés, affiches)
src/web/source.rs     Wikidata, l'instantané d'un site statique, ou la sélection embarquée
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
| `GET /api/status` | la source des titres (`wikidata` ou `selection`), leur nombre et les plateformes |
| `GET /api/picks?platform=prime-video&kind=film&seed=42` | 5 titres au hasard dans tout le catalogue (la sélection du jour sans `seed`) |
| `GET /api/titles/:key` | un titre complet : `wd:Q83495` (Wikidata) ou `local:12` |
| `GET /api/catalogue?platform=netflix&kind=serie&page=2` | une page (20 titres) du catalogue d'une plateforme |
| `POST /api/recommendations` | `{"liked":[{"key":"wd:Q83495","name":"Matrix","kind":"film","genres":["action","sf"],"platforms":["netflix"]}],"platforms":["netflix"],"genres":["drame"],"kind":"serie","limit":6}` |
| `GET /api/titles` | la sélection embarquée (le secours hors ligne) |
| `GET /api/votes/stats` | les votes de la communauté, par titre |
| `POST /api/votes` | `{"title":"wd:Q83495","liked":true}` ; changer d'avis : `{"title":"wd:Q83495","previous":true,"liked":false}` |
| `POST /api/proposals` | `{"title":"Severance","platform":"prime-video"}` |
| `GET /health` | l'état du service |

Plateformes : `netflix`, `prime-video`, `disney-plus`, `hbo-max`. Formats : `film`, `serie`.

## Tout faire depuis un téléphone

Aucun ordinateur n'est nécessaire : GitHub Actions compile, teste, fusionne et déploie.

### 1. Fusion automatique dans `master`

Chaque push sur une branche lance la CI (format, clippy du serveur et des îlots, tests, compilation WebAssembly, export statique). **Quand tout est vert, la branche est fusionnée dans `master`** (sauf les branches `wip/…`), puis Heroku, GitHub Pages et l'app iOS sont déployés.

> Si les dépôts de Paquets sont privés : ajoutez un secret **`REPOS_TOKEN`** (un jeton GitHub en lecture seule sur ces dépôts) dans *Settings → Secrets and variables → Actions*.

### 2. Les données des films et séries

Rien à configurer : le catalogue vient de [Wikidata](https://www.wikidata.org/) (par le moteur SPARQL [QLever](https://qlever.dev/), ou le service de Wikidata en secours), les résumés et les affiches de [Wikipédia](https://fr.wikipedia.org/). Le catalogue est chargé au déploiement (`cineshuffle catalogue`), puis rechargé chaque jour ; les fiches sont chargées à la demande et gardées une semaine.

Wikidata indique les titres *associés* à chaque plateforme (ses originaux, et ce qu'elle distribue) : la disponibilité en France à un instant donné peut varier.

### 3. GitHub Pages

Une seule fois, dans l'app GitHub ou sur github.com : **Settings → Pages → Source : « GitHub Actions »**. Le workflow *GitHub Pages* publie ensuite le site à chaque fusion, et chaque nuit (nouvelle sélection du jour) : `https://maxlestage.github.io/CineShuffle/`.

Sur GitHub Pages il n'y a pas de serveur. Ajoutez la variable **`CINESHUFFLE_API_URL`** (l'adresse de l'app Heroku) : le site appelle alors l'API Heroku et tire dans tout le catalogue. Sinon, il tire dans l'instantané publié avec lui (les 60 titres les plus connus de chaque plateforme et de chaque format, avec résumé et affiche) et les votes restent sur l'appareil.

### 4. Heroku (buildpack `emk/rust`)

Dans *Settings → Secrets and variables → Actions* :

| Où | Nom | Valeur |
|---|---|---|
| Secret | `HEROKU_API_KEY` | Heroku → *Account settings* → *API Key* → *Reveal* |
| Variable | `HEROKU_APP_NAME` | le nom de l'app (créée automatiquement si elle n'existe pas) |
| Variable (facultatif) | `HEROKU_REGION` | `eu` (par défaut) ou `us` |
| Variable (facultatif) | `HEROKU_POSTGRES_PLAN` | par exemple `essential-0`, pour garder les votes dans PostgreSQL |
| Variable (facultatif) | `CORS_ORIGINS` | les sites autorisés à appeler l'API (par défaut `https://maxlestage.github.io`) |

Puis *Actions → Heroku → Run workflow* (ou un simple push). Le workflow crée l'app sur la pile `heroku-26`, installe le buildpack **`emk/rust`**, envoie le code et suit la compilation : `RustConfig` installe Rust, `heroku-build.sh` compile le serveur et les îlots, le `Procfile` démarre `target/release/cineshuffle`.

Si le dépôt est public, le bouton marche aussi :

[![Deploy to Heroku](https://www.herokucdn.com/deploy/button.svg)](https://www.heroku.com/deploy?template=https://github.com/maxlestage/CineShuffle)

### 5. L'app iOS sur TestFlight

Le workflow *iOS* génère le projet avec XcodeGen, compile et lance les tests sur un simulateur, sur un Mac de GitHub. Pour l'installer sur votre iPhone avec TestFlight :

1. Sur [developer.apple.com](https://developer.apple.com/account) (compte Apple Developer), notez votre **Team ID** (*Membership*).
2. Sur [App Store Connect](https://appstoreconnect.apple.com) : créez l'app (identifiant `com.maxlestage.cineshuffle`), puis *Utilisateurs et accès → Intégrations → Clés d'API* : créez une clé (rôle *Admin*), téléchargez le `.p8`, notez son **Key ID** et l'**Issuer ID**.
3. Dans le dépôt, ajoutez les secrets `APPLE_TEAM_ID`, `ASC_KEY_ID`, `ASC_ISSUER_ID` et `ASC_KEY_P8` (le contenu du `.p8`), et la variable `CINESHUFFLE_API_URL` (l'adresse de l'app Heroku).
4. *Actions → iOS → Run workflow* : la build arrive dans TestFlight quelques minutes plus tard.

## Sur un ordinateur (facultatif)

```sh
cargo install --git https://github.com/maxlestage/Paquets paquets --locked
paquets dev                                   # http://localhost:3000, îlots compris
cargo test                                    # catalogue, tirage, suggestions, Wikidata, pages, API
cargo test -- --ignored live                  # Wikidata et Wikipédia en direct
cargo run -- catalogue                        # le catalogue Wikidata dans data/catalogue.json
BASE_PATH=/CineShuffle cargo run -- export dist  # le site statique
cargo run -- catalog-json > ios/CineShuffle/catalog.json   # après un changement du catalogue
cd ios && xcodegen generate && open CineShuffle.xcodeproj  # l'app iOS (macOS)
```

## Mentions

Catalogue : [Wikidata](https://www.wikidata.org/) (licence CC0). Résumés : [Wikipédia](https://fr.wikipedia.org/) (licence CC BY-SA), chaque fiche renvoie à son article. Affiches : illustrations des articles de Wikipédia, propriété de leurs ayants droit.

Netflix, Prime Video, Disney+ et HBO Max sont des marques de leurs propriétaires respectifs ; CineShuffle n'est affilié à aucune d'elles. Les résumés et les affiches dessinées de la sélection embarquée sont originaux.
