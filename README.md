# LCH Lab

A richer Axum + Askama + HTMX playground for experimenting with both OKLCH and CIELAB-style LCH color spaces.

## Features

- Dual-mode slider stack that toggles between perceptual OKLCH and a clearly labeled classic LCH approximation.
- Shareable URLs that persist slider, mode, and contrast-input state without any custom JavaScript framework.
- Rich color outputs (oklch()/lch(), rgb(), hex, hsl) with quick copy buttons.
- WCAG contrast checker that compares the swatch to custom foreground/background colors, including AA/AAA badges.
- Lightweight visualization lab with 2D L–C and C–H heatmaps plus a rotating pseudo-3D point cloud rendered on `<canvas>`.

## Running locally

1. Install the Rust toolchain via [rustup](https://rustup.rs/) if you haven't already.
2. From this directory run:

```bash
cargo run
```

3. Open <http://127.0.0.1:3000> and drag the sliders. HTMX calls `/preview` with the current values so the swatch, outputs, contrast cards, and visualizations update without a full page reload.

## Development notes

- Templates live in `templates/` and are rendered with Askama. The preview route clamps incoming values to supported ranges before generating CSS strings for both color models.
- Visualization slices and the point cloud are generated server-side so HTMX swaps can remain HTML-only. A tiny canvas helper animates the pseudo-3D projection.
- Format with `cargo fmt` and validate with `cargo check` before committing changes.
