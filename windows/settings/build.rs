// The engine's version (engine/Cargo.toml), for the About page.
fn main() {
    let toml = std::fs::read_to_string("../../engine/Cargo.toml").unwrap_or_default();
    let version = toml
        .lines()
        .find_map(|l| l.strip_prefix("version = \"").and_then(|v| v.strip_suffix('"')))
        .unwrap_or("");
    println!("cargo:rustc-env=BADA_ENGINE_VERSION={version}");
    println!("cargo:rerun-if-changed=../../engine/Cargo.toml");
}