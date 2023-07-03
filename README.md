# Rustbolt

Readme WIP

## Bootstrap

### Configuration

* copy config.template.toml as config.toml and setup as desired

#### For Cartographer

* rustup target install wasm32-unknown-unknown
* cargo install wasm-server-runner
* cargo install -f wasm-bindgen-cli
* put in .cargo/config.toml:
```
[target.wasm32-unknown-unknown]
runner = "wasm-server-runner"
```
* ln -s . assets # Bevy looks into an "assets" folder for the assets
* cargo run --target wasm32-unknown-unknown --bin cartographer

To build a release:

* cargo build --release --target wasm32-unknown-unknown
* wasm-bindgen --out-dir ./out/ --target web ./target/

### DBC files

* mkdir -p data/dbcs
* cargo run --bin dbc_extractor -- -c ~/PATH-TO-CLIENT -o data/dbcs

### Terrain files (geometry and liquid)

* mkdir -p data/terrain
* cargo run --bin terrain_extractor -- -c ~/PATH-TO-CLIENT -d data/dbcs -o data/terrain

## Development

### Run tests

```bash
$ cargo test
```
