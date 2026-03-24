//! Validate docs/tools_registry.json at build time (single source of truth for tool names; descriptions synced to handler).

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let json_path = Path::new(&manifest_dir)
        .join("docs")
        .join("tools_registry.json");
    let json_str = fs::read_to_string(&json_path).expect("read tools_registry.json");
    let _: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).expect("parse tools_registry.json");
}
