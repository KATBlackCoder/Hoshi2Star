Parser for Wolf RPG Editor map files
====================================
[<img alt="github" src="https://img.shields.io/badge/github-G1org1owo/wolfrpg--map--parser-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/G1org1owo/wolfrpg-map-parser)
[<img alt="Crates.io Version" src="https://img.shields.io/crates/v/wolfrpg-map-parser?style=for-the-badge" height="20">](https://crates.io/crates/wolfrpg-map-parser)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-wolfprg--map--parser-66c2a5?style=for-the-badge" height="20">](https://docs.rs/wolfrpg-map-parser)
[<img alt="License" src="https://img.shields.io/crates/l/wolfrpg-map-parser?style=for-the-badge" height="20">](https://github.com/G1org1owo/wolfrpg-map-parser/blob/main/LICENSE)

The aim of this crate is to allow users to easily parse Wolf RPG Editor map (`.mps`) files and expose a complete 
interface to enable interaction with each component of a map, from the tiles to the events.

This package includes both a library crate that parses the map into a tree of rust structs and a binary crate that
outputs the result in JSON format.

## Usage
You can run the standalone directly through Cargo:
```bash
$ cargo run --project wolfrpg-map-parser --bin wolfrpg-map-parser --features="serde" <filepath>
```

Or you can add the crate and import the needed modules:

```rust
use wolfrpg_map_parser::Map;

fn main() {
    match fs::read("filepath.mps") {
        Ok(bytes) => {
            let map: Map = Map::parse(&bytes);

            // Data manipulation ...
        }
        Err(_) => {
            // Error handling ...
        }
    }
}
```