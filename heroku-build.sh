#!/usr/bin/env bash
# Compile le serveur et les îlots (WebAssembly). Lancé sur Heroku par RustConfig une
# fois que le buildpack emk/rust a installé Rust ; marche aussi à la main : ./heroku-build.sh
# Le Procfile démarre target/release/reelpick.
set -euo pipefail

# Si les dépôts de Paquets sont privés, Cargo les récupère avec git, qui demande les
# identifiants GitHub à un assistant lisant la variable REPOS_TOKEN (un jeton GitHub
# qui peut les lire ; le buildpack donne les variables de configuration dans ENV_DIR).
# Le jeton n'apparaît dans aucune URL.
if [ -z "${REPOS_TOKEN:-}" ] && [ -n "${ENV_DIR:-}" ] && [ -f "$ENV_DIR/REPOS_TOKEN" ]; then
  REPOS_TOKEN=$(<"$ENV_DIR/REPOS_TOKEN")
fi
if [ -n "${REPOS_TOKEN:-}" ]; then
  export REPOS_TOKEN CARGO_NET_GIT_FETCH_WITH_CLI=true GIT_CONFIG_COUNT=1 \
    GIT_CONFIG_KEY_0=credential.https://github.com.helper \
    GIT_CONFIG_VALUE_0='!f() { test "$1" = get && echo username=x-access-token && echo "password=$REPOS_TOKEN"; }; f'
fi
cd "$(dirname "$0")"
# Sur Heroku, Cargo compile dans le cache du buildpack (CARGO_TARGET_DIR).
target="${CARGO_TARGET_DIR:-target}"

echo "-----> Compilation du serveur"
cargo build --release --locked --bin reelpick
mkdir -p target/release
if [ "$target" != target ]; then
  cp "$target/release/reelpick" target/release/
fi
echo "       target/release/reelpick prêt"

echo "-----> Compilation des îlots (WebAssembly)"
rustup target add wasm32-unknown-unknown
cargo build --locked --lib --target wasm32-unknown-unknown --profile wasm-release

# wasm-bindgen doit être à la version de Cargo.lock : un binaire précompilé, ou
# compilé s'il ne peut pas être téléchargé.
version=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
bin="$target/wasm-bindgen-$version"
if [ ! -x "$bin/wasm-bindgen" ]; then
  mkdir -p "$bin"
  curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}/wasm-bindgen-${version}-x86_64-unknown-linux-musl.tar.gz" \
    | tar -xz --strip-components=1 -C "$bin" \
    || cargo install wasm-bindgen-cli --version "$version" --locked --root "$bin" --bin wasm-bindgen
  [ -x "$bin/wasm-bindgen" ] || mv "$bin/bin/wasm-bindgen" "$bin/"
fi
"$bin/wasm-bindgen" --target web --no-typescript --remove-name-section --remove-producers-section \
  --out-dir pkg \
  "$target/wasm32-unknown-unknown/wasm-release/reelpick.wasm"
echo "       îlots prêts dans pkg/"
