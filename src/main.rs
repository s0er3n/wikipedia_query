use select::document::Document;
use select::predicate::{Attr, Name};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
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

fn make_query(target: &str) -> Wiki {
    let resp_txt = reqwest::blocking::get(format!("https://en.wikipedia.org/wiki/{}", target))
        .unwrap()
        .text()
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
// TODO: add rocket framework
fn main() {
    let mut cache = Cache {
        articles: HashMap::new(),
        article_count: HashMap::new(),
    };
    let wiki = make_query("a");

    cache.add(wiki);
    println!("wiki = {:#?}", &cache);
}
