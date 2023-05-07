# Rustbolt

Readme WIP

## Bootstrap

### Configuration

* copy config.template.toml as config.toml and setup as desired

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
