use axum::extract::Path;

use http::Method;
use tower_http::cors::{Any, CorsLayer};

use select::document::Document;
use select::predicate::{Attr, Name};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
struct Wiki {
    title: String,
    url_ending: String,
    content_html: String,
    links: HashSet<String>,
}

#[derive(Debug)]
struct Cache {
    articles: Mutex<HashMap<String, Wiki>>,
    article_count: Mutex<HashMap<String, usize>>,
}

impl Cache {
    fn add(&self, wiki: Wiki) {
        let url_ending = wiki.url_ending.clone();
        self.articles
            .lock()
            .unwrap()
            .insert(url_ending.clone(), wiki);
        self.article_count
            .lock()
            .unwrap()
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
    let cache = Arc::new(Cache {
        articles: Mutex::new(HashMap::new()),
        article_count: Mutex::new(HashMap::new()),
    });

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);
    // build our application with a route
    let app = Router::new()
        .route(
            "/article/:target",
            get({
                let cache = Arc::clone(&cache);
                move |path| get_article(path, cache)
            }),
        )
        .with_state(cache)
        .layer(cors);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_article(Path(target): Path<String>, cache: Arc<Cache>) -> impl IntoResponse {
    let cache = cache;
    if cache.articles.lock().unwrap().contains_key(&target) {
        let wiki = cache.articles.lock().unwrap().get(&target).unwrap().clone();
        cache
            .article_count
            .lock()
            .unwrap()
            .entry(target)
            .and_modify(|e| *e += 1)
            .or_insert(1);
        return (StatusCode::CREATED, Json(wiki));
    } else {
        let wiki = make_query(&target).await;
        cache.add(wiki.clone());
        return (StatusCode::CREATED, Json(wiki));
    }
}
