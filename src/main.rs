use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::Query, routing::get, Router};
use csscolorparser::parse;
use serde::Deserialize;
use std::net::SocketAddr;

const DEFAULT_L: f64 = 0.72;
const DEFAULT_C: f64 = 0.14;
const DEFAULT_H: f64 = 220.0;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/preview", get(preview));

    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();
    println!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");

    axum::serve(listener, app).await.expect("server error");
}

async fn index(query: Option<Query<PreviewQuery>>) -> impl IntoResponse {
    let params = query
        .map(|Query(query)| ColorParams::from_query(query))
        .unwrap_or_default();

    IndexTemplate {
        params,
        presets: &PRESETS,
        hex_value: params.hex_color(),
    }
}

async fn preview(Query(query): Query<PreviewQuery>) -> impl IntoResponse {
    let params = ColorParams::from_query(query);

    PreviewTemplate {
        params,
        hex_value: params.hex_color(),
    }
}

#[derive(Deserialize, Debug)]
struct PreviewQuery {
    l: Option<f64>,
    c: Option<f64>,
    h: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
struct ColorParams {
    l: f64,
    c: f64,
    h: f64,
}

impl ColorParams {
    fn from_query(query: PreviewQuery) -> Self {
        Self {
            l: clamp(query.l, DEFAULT_L, 0.0, 1.0),
            c: clamp(query.c, DEFAULT_C, 0.0, 0.4),
            h: clamp(query.h, DEFAULT_H, 0.0, 360.0),
        }
    }

    pub fn css_color(&self) -> String {
        format!("oklch({:.2} {:.3} {:.0})", self.l, self.c, self.h)
    }

    pub fn l_display(&self) -> String {
        format!("{:.2}", self.l)
    }

    pub fn c_display(&self) -> String {
        format!("{:.3}", self.c)
    }

    pub fn h_display(&self) -> String {
        format!("{:.0}", self.h)
    }

    fn hex_color(&self) -> Option<String> {
        let css = self.css_color();
        let color = parse(&css).ok()?;
        let [r, g, b, _] = color.to_rgba8();
        Some(format!("#{:02X}{:02X}{:02X}", r, g, b))
    }
}

impl Default for ColorParams {
    fn default() -> Self {
        Self {
            l: DEFAULT_L,
            c: DEFAULT_C,
            h: DEFAULT_H,
        }
    }
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
    hex_value: Option<String>,
}

#[derive(Template)]
#[template(path = "preview.html")]
struct PreviewTemplate {
    params: ColorParams,
    hex_value: Option<String>,
}

fn clamp(value: Option<f64>, default: f64, min: f64, max: f64) -> f64 {
    value.unwrap_or(default).clamp(min, max)
}
