# Bundle Sizes

Default `cargo tauri build` output:

1. **DMG size**: 10 MB
2. **Binary size**: 25.9 MB

Optimized `cargo tauri build` output:

```
cp src-tauri/.cargo/config-optimized.toml src-tauri/.cargo/config.toml
cargo tauri build
rm src-tauri/.cargo/config.toml
```

1. **DMG size**: 4.9 MB
2. **Binary size**: 8.7 MB
