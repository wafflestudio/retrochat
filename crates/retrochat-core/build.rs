use std::path::Path;

fn main() {
    // Force recompilation when any migration file changes
    let migrations_dir = Path::new("migrations");
    if migrations_dir.exists() {
        println!("cargo:rerun-if-changed=migrations");
        if let Ok(entries) = std::fs::read_dir(migrations_dir) {
            for entry in entries.flatten() {
                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
        }
    }
}
