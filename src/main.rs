use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::Query, routing::get, Router};
use csscolorparser::{parse, Color};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

const DEFAULT_L: f64 = 0.72;
const DEFAULT_C: f64 = 0.14;
const DEFAULT_H: f64 = 220.0;
const DEFAULT_FG: &str = "#0F1419";
const DEFAULT_BG: &str = "#FFFFFF";
const MAX_LIGHTNESS: f64 = 1.0;
const MAX_CHROMA: f64 = 0.4;
const MAX_HUE: f64 = 360.0;
const LCH_C_SCALE: f64 = 150.0;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/preview", get(preview));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    println!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");

    axum::serve(listener, app).await.expect("server error");
}

async fn index(query: Option<Query<PreviewQuery>>) -> impl IntoResponse {
    let query = query.map(|Query(query)| query).unwrap_or_default();
    build_index_template(query)
}

async fn preview(Query(query): Query<PreviewQuery>) -> impl IntoResponse {
    build_preview_template(query)
}

fn build_index_template(query: PreviewQuery) -> IndexTemplate {
    let params = ColorParams::from_query(&query);
    let view_mode = ViewMode::from_param(query.view.as_deref());
    let (fg_color, fg_value) = sanitize_user_color(query.fg.as_deref(), DEFAULT_FG);
    let (bg_color, bg_value) = sanitize_user_color(query.bg.as_deref(), DEFAULT_BG);
    let swatch_color = params.parsed_color();
    let viz = VisualizationContext::new(params, view_mode);
    let contrast = ContrastChecker::new(swatch_color, &fg_value, &bg_value);

    IndexTemplate {
        params,
        presets: &PRESETS,
        fg_color,
        bg_color,
        contrast,
        viz,
    }
}

fn build_preview_template(query: PreviewQuery) -> PreviewTemplate {
    let params = ColorParams::from_query(&query);
    let view_mode = ViewMode::from_param(query.view.as_deref());
    let (fg_color, fg_value) = sanitize_user_color(query.fg.as_deref(), DEFAULT_FG);
    let (bg_color, bg_value) = sanitize_user_color(query.bg.as_deref(), DEFAULT_BG);
    let swatch_color = params.parsed_color();
    let viz = VisualizationContext::new(params, view_mode);
    let contrast = ContrastChecker::new(swatch_color, &fg_value, &bg_value);

    PreviewTemplate {
        fg_color,
        bg_color,
        contrast,
        viz,
    }
}

#[derive(Deserialize, Debug, Default)]
struct PreviewQuery {
    l: Option<f64>,
    c: Option<f64>,
    h: Option<f64>,
    mode: Option<String>,
    view: Option<String>,
    fg: Option<String>,
    bg: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorMode {
    Oklch,
    Lch,
}

impl ColorMode {
    fn from_param(value: Option<&str>) -> Self {
        match value {
            Some(v) if v.eq_ignore_ascii_case("lch") => Self::Lch,
            _ => Self::Oklch,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Oklch => "OKLCH",
            Self::Lch => "Classic LCH",
        }
    }

    fn param_value(&self) -> &'static str {
        match self {
            Self::Oklch => "oklch",
            Self::Lch => "lch",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Oklch => "CSS Color 4 perceptually uniform OKLab flavor",
            Self::Lch => "CIELAB-inspired approximation (limited chroma in this UI)",
        }
    }
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::Oklch
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewMode {
    Single,
    Compare,
}

impl ViewMode {
    fn from_param(value: Option<&str>) -> Self {
        match value {
            Some(v) if v.eq_ignore_ascii_case("compare") => Self::Compare,
            _ => Self::Single,
        }
    }

    fn param_value(&self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Compare => "compare",
        }
    }

    fn is_compare(&self) -> bool {
        matches!(self, Self::Compare)
    }
}

impl Default for ViewMode {
    fn default() -> Self {
        Self::Single
    }
}

#[derive(Clone, Copy, Debug)]
struct ColorParams {
    l: f64,
    c: f64,
    h: f64,
    mode: ColorMode,
}

impl ColorParams {
    fn from_query(query: &PreviewQuery) -> Self {
        Self {
            l: clamp(query.l, DEFAULT_L, 0.0, MAX_LIGHTNESS),
            c: clamp(query.c, DEFAULT_C, 0.0, MAX_CHROMA),
            h: clamp(query.h, DEFAULT_H, 0.0, MAX_HUE),
            mode: ColorMode::from_param(query.mode.as_deref()),
        }
    }

    fn css_color(&self) -> String {
        match self.mode {
            ColorMode::Oklch => self.oklch_css(),
            ColorMode::Lch => self.lch_css(),
        }
    }

    fn oklch_css(&self) -> String {
        format!("oklch({:.3} {:.3} {:.0})", self.l, self.c, self.h)
    }

    fn lch_css(&self) -> String {
        let l = (self.l * 100.0).clamp(0.0, 100.0);
        let c = (self.c * LCH_C_SCALE).clamp(0.0, LCH_C_SCALE);
        format!("lch({:.1}% {:.1} {:.0})", l, c, self.h)
    }

    fn l_display(&self) -> String {
        format!("{:.2}", self.l)
    }

    fn c_display(&self) -> String {
        format!("{:.3}", self.c)
    }

    fn h_display(&self) -> String {
        format!("{:.0}", self.h)
    }

    fn parsed_color(&self) -> Option<Color> {
        parse(&self.css_color()).ok()
    }
}

impl Default for ColorParams {
    fn default() -> Self {
        Self {
            l: DEFAULT_L,
            c: DEFAULT_C,
            h: DEFAULT_H,
            mode: ColorMode::Oklch,
        }
    }
}

#[derive(Clone)]
struct ModeOutputs {
    css: String,
    hex: Option<String>,
    rgb: Option<String>,
    hsl: Option<String>,
}

impl ModeOutputs {
    fn new(params: ColorParams) -> Self {
        let css = params.css_color();
        let parsed = parse(&css).ok();
        let hex = parsed.as_ref().map(color_to_hex);
        let rgb = parsed.as_ref().map(|color| rgb_string(color));
        let hsl = parsed.as_ref().map(|color| hsl_string(color));

        Self { css, hex, rgb, hsl }
    }
}

#[derive(Clone)]
struct ModePanelData {
    mode: ColorMode,
    params: ColorParams,
    outputs: ModeOutputs,
}

impl ModePanelData {
    fn new(base: &ColorParams, mode: ColorMode) -> Self {
        let mut params = *base;
        params.mode = mode;
        Self {
            mode,
            params,
            outputs: ModeOutputs::new(params),
        }
    }

    fn css_dom_id(&self) -> String {
        format!("css-{}", self.mode.param_value())
    }

    fn chroma_note(&self) -> &'static str {
        match self.mode {
            ColorMode::Oklch => "(OK units)",
            ColorMode::Lch => "(≈CIE)",
        }
    }

    fn chroma_value_display(&self) -> String {
        match self.mode {
            ColorMode::Oklch => self.params.c_display(),
            ColorMode::Lch => format!("{:.0}", self.params.c * LCH_C_SCALE),
        }
    }
}

#[derive(Clone)]
struct ContrastChecker {
    swatch_as_background: Option<ContrastSummary>,
    swatch_as_foreground: Option<ContrastSummary>,
}

impl ContrastChecker {
    fn new(swatch: Option<Color>, fg: &Color, bg: &Color) -> Self {
        if let Some(swatch_color) = swatch {
            let swatch_as_background = Some(ContrastSummary::new(fg, &swatch_color));
            let swatch_as_foreground = Some(ContrastSummary::new(&swatch_color, bg));
            Self {
                swatch_as_background,
                swatch_as_foreground,
            }
        } else {
            Self {
                swatch_as_background: None,
                swatch_as_foreground: None,
            }
        }
    }
}

#[derive(Clone)]
struct ContrastSummary {
    ratio: f64,
    aa_normal: bool,
    aa_large: bool,
    aaa_normal: bool,
    aaa_large: bool,
}

impl ContrastSummary {
    fn new(foreground: &Color, background: &Color) -> Self {
        let ratio = contrast_ratio(foreground, background);
        Self {
            ratio,
            aa_normal: ratio >= 4.5,
            aa_large: ratio >= 3.0,
            aaa_normal: ratio >= 7.0,
            aaa_large: ratio >= 4.5,
        }
    }

    fn ratio_display(&self) -> String {
        format!("{:.2}:1", self.ratio)
    }
}

#[derive(Clone)]
struct VisualizationContext {
    view_mode: ViewMode,
    active_mode: ColorMode,
    panels: Vec<ModePanelData>,
    plot_json: String,
}

impl VisualizationContext {
    fn new(params: ColorParams, view_mode: ViewMode) -> Self {
        let panels = vec![
            ModePanelData::new(&params, ColorMode::Oklch),
            ModePanelData::new(&params, ColorMode::Lch),
        ];
        let plot_json = build_plot_payload(&panels);
        Self {
            view_mode,
            active_mode: params.mode,
            panels,
            plot_json,
        }
    }

    fn active_panel(&self) -> &ModePanelData {
        self.panel_for(self.active_mode)
            .or_else(|| self.panels.first())
            .expect("at least one color model should be present")
    }

    fn panel_for(&self, mode: ColorMode) -> Option<&ModePanelData> {
        self.panels.iter().find(|panel| panel.mode == mode)
    }

    fn active_outputs(&self) -> &ModeOutputs {
        &self.active_panel().outputs
    }

    fn max_lightness(&self) -> f64 {
        MAX_LIGHTNESS
    }

    fn max_chroma(&self) -> f64 {
        MAX_CHROMA
    }

    fn max_hue(&self) -> f64 {
        MAX_HUE
    }

    fn max_lch_chroma(&self) -> f64 {
        MAX_CHROMA * LCH_C_SCALE
    }

    fn max_lch_chroma_display(&self) -> String {
        format!("{:.0}", self.max_lch_chroma())
    }
}

#[derive(Serialize)]
struct VizPoint {
    l: f64,
    c: f64,
    h: f64,
    css: String,
}

#[derive(Serialize)]
struct PlotDataset {
    mode: &'static str,
    label: &'static str,
    points: Vec<VizPoint>,
}

#[derive(Serialize)]
struct SelectedPoint {
    mode: &'static str,
    label: &'static str,
    l: f64,
    c: f64,
    h: f64,
    css: String,
}

#[derive(Serialize)]
struct PlotPayload {
    datasets: Vec<PlotDataset>,
    selections: Vec<SelectedPoint>,
}

#[derive(Clone, Copy)]
struct Preset {
    name: &'static str,
    l: f64,
    c: f64,
    h: f64,
}

const PRESETS: [Preset; 4] = [
    Preset {
        name: "Sky",
        l: 0.88,
        c: 0.08,
        h: 220.0,
    },
    Preset {
        name: "Indigo",
        l: 0.72,
        c: 0.15,
        h: 260.0,
    },
    Preset {
        name: "Rose",
        l: 0.74,
        c: 0.17,
        h: 20.0,
    },
    Preset {
        name: "Mint",
        l: 0.82,
        c: 0.11,
        h: 160.0,
    },
];

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    params: ColorParams,
    presets: &'static [Preset],
    fg_color: String,
    bg_color: String,
    contrast: ContrastChecker,
    viz: VisualizationContext,
}

#[derive(Template)]
#[template(path = "preview.html")]
struct PreviewTemplate {
    fg_color: String,
    bg_color: String,
    contrast: ContrastChecker,
    viz: VisualizationContext,
}

fn clamp(value: Option<f64>, default: f64, min: f64, max: f64) -> f64 {
    value.unwrap_or(default).clamp(min, max)
}

fn sanitize_user_color(input: Option<&str>, fallback: &str) -> (String, Color) {
    if let Some(value) = input {
        if let Some(result) = parse_user_color(value) {
            return result;
        }
    }
    parse_user_color(fallback).expect("fallback color must parse")
}

fn parse_user_color(value: &str) -> Option<(String, Color)> {
    let color = parse(value).ok()?;
    let hex = color_to_hex(&color);
    Some((hex, color))
}

fn color_to_hex(color: &Color) -> String {
    let [r, g, b, _] = color.to_rgba8();
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn rgb_string(color: &Color) -> String {
    let [r, g, b, _] = color.to_rgba8();
    format!("rgb({} {} {})", r, g, b)
}

fn hsl_string(color: &Color) -> String {
    let (h, s, l) = rgb_to_hsl(color.r as f64, color.g as f64, color.b as f64);
    format!("hsl({:.0} {:.0}% {:.0}%)", h, s * 100.0, l * 100.0)
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let mut h = 0.0;
    let l = (max + min) / 2.0;
    let delta = max - min;

    let s = if delta == 0.0 {
        0.0
    } else {
        if l > 0.5 {
            delta / (2.0 - max - min)
        } else {
            delta / (max + min)
        }
    };

    if delta != 0.0 {
        h = if max == r {
            ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            ((b - r) / delta) + 2.0
        } else {
            ((r - g) / delta) + 4.0
        } * 60.0;
    }

    if h < 0.0 {
        h += 360.0;
    }

    (h, s, l)
}

fn contrast_ratio(foreground: &Color, background: &Color) -> f64 {
    let l1 = relative_luminance(foreground);
    let l2 = relative_luminance(background);
    let (bright, dark) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (bright + 0.05) / (dark + 0.05)
}

fn relative_luminance(color: &Color) -> f64 {
    fn expand(channel: f32) -> f64 {
        let c = channel as f64;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * expand(color.r) + 0.7152 * expand(color.g) + 0.0722 * expand(color.b)
}

fn build_plot_payload(panels: &[ModePanelData]) -> String {
    let datasets = [ColorMode::Oklch, ColorMode::Lch]
        .into_iter()
        .map(|mode| PlotDataset {
            mode: mode.param_value(),
            label: mode.label(),
            points: build_point_cloud(mode),
        })
        .collect::<Vec<_>>();

    let selections = [ColorMode::Oklch, ColorMode::Lch]
        .into_iter()
        .filter_map(|mode| panels.iter().find(|panel| panel.mode == mode))
        .map(|panel| SelectedPoint {
            mode: panel.mode.param_value(),
            label: panel.mode.label(),
            l: panel.params.l,
            c: panel.params.c,
            h: panel.params.h,
            css: panel.outputs.css.clone(),
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&PlotPayload {
        datasets,
        selections,
    })
    .unwrap_or_else(|_| "{}".to_string())
}

fn build_point_cloud(mode: ColorMode) -> Vec<VizPoint> {
    let l_values = [0.12, 0.25, 0.4, 0.55, 0.7, 0.85];
    let c_values = [0.02, 0.1, 0.18, 0.26, 0.34];
    let mut points = Vec::new();
    for &l in &l_values {
        for &c in &c_values {
            for h in (0..360).step_by(40) {
                let params = ColorParams {
                    l,
                    c,
                    h: h as f64,
                    mode,
                };
                points.push(VizPoint {
                    l,
                    c,
                    h: h as f64,
                    css: params.css_color(),
                });
            }
        }
    }
    points
}
