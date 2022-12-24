use axum::extract::Path;
use select::document::Document;
use select::predicate::{Attr, Name};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
struct Wiki {
    title: String,
    url_ending: String,
    content_html: String,
    links: HashSet<String>,
}

#[derive(Debug)]
struct Cache {
    articles: HashMap<String, Wiki>,
    article_count: HashMap<String, usize>,
}

impl Cache {
    fn add(&mut self, wiki: Wiki) {
        let url_ending = wiki.url_ending.clone();
        self.articles.insert(url_ending.clone(), wiki);
        self.article_count
            .entry(url_ending)
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }
}

async fn make_query(target: &str) -> Wiki {
    let resp_txt = reqwest::get(format!("https://en.wikipedia.org/wiki/{}", target))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let wiki_page = Document::from(resp_txt.as_str());

    let title = wiki_page.find(Name("h1")).next().unwrap().text();

    let article = wiki_page
        .find(Attr("id", "mw-content-text"))
        .next()
        .unwrap();

    let links: HashSet<String> = article
        .find(Name("a"))
        .filter_map(|n| n.attr("href"))
        .filter(|l| {
            if l.starts_with("/wiki/Help") || l.starts_with("/wiki/File") {
                return false;
            };
            if l.starts_with("/wiki/") {
                return true;
            };
            false
        })
        .map(|x| String::from(&x[6..]))
        .collect();
    let content = article.html();

    return Wiki {
        title,
        url_ending: target.to_string(),
        content_html: content,
        links,
    };
}

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // initialize tracing

    // build our application with a route
    let app = Router::new().route("/article/:target", get(get_article));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_article(Path(target): Path<String>) -> impl IntoResponse {
    (StatusCode::CREATED, Json(make_query(&target).await))
}
