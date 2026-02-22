use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::Query, routing::get, Router};
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

async fn index() -> impl IntoResponse {
    IndexTemplate {
        params: ColorParams::default(),
    }
}

async fn preview(Query(query): Query<PreviewQuery>) -> impl IntoResponse {
    let params = ColorParams::from_query(query);

    PreviewTemplate { params }
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

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    params: ColorParams,
}

#[derive(Template)]
#[template(path = "preview.html")]
struct PreviewTemplate {
    params: ColorParams,
}

fn clamp(value: Option<f64>, default: f64, min: f64, max: f64) -> f64 {
    value.unwrap_or(default).clamp(min, max)
}
