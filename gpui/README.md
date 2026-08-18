# Dagsverk GPUI Preview

This workspace contains the native Rust and GPUI port. The Angular and Electron application remains the parity reference.

## Run

```bash
cd gpui
cargo run
```

The default command uses the separate `Dagsverk GPUI Preview` data directory. Use a copied fixture database during development:

```bash
cargo run -- --database /absolute/path/to/copied-dagsverk.db
cargo run -- --component-gallery
```

`--compatibility-mode` uses the stable Dagsverk data path. Close Electron Dagsverk before you use this mode.

The preview uses GPUI 0.2.2 and Rust 1.96.0.
