# OKLCH Lab

A tiny Axum + Askama + HTMX playground for experimenting with OKLCH colors.

## Running locally

1. Install the Rust toolchain via [rustup](https://rustup.rs/) if you haven't already.
2. From this directory run:

```bash
cargo run
```

3. Open <http://127.0.0.1:3000> and drag the sliders. HTMX will call `/preview` with the current values so the swatch updates without a full page reload.

## Development notes

- Templates live in `templates/` and are rendered with Askama.
- The preview route clamps incoming values to the supported OKLCH ranges before rendering the CSS `oklch()` string.
- Format the codebase with `cargo fmt` and check it with `cargo check` before committing changes.
