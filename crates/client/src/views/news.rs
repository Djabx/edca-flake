mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use crate::views::ViewEvent;

pub struct NewsView {
    pub articles: Vec<Article>,
    pub loading: bool,
    pub error: Option<String>,
    pub scroll: usize,
    rx: Option<std::sync::mpsc::Receiver<Result<Vec<Article>, String>>>,
}

pub struct Article {
    pub title: String,
    pub date: String,
    pub text: String,
}

impl NewsView {
    pub fn new() -> Self {
        Self {
            articles: Vec::new(),
            loading: true,
            error: None,
            scroll: 0,
            rx: None,
        }
    }

    /// Starts the background fetch. No-ops if a fetch is already in progress or articles are loaded.
    pub fn start_fetch(&mut self, api: &crate::api_client::ApiClient) {
        if self.rx.is_some() || !self.loading {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let client = api.http_client();
        api.spawn(async move {
            let _ = tx.send(fetch_articles_async(client).await);
        });
        self.rx = Some(rx);
    }

    pub fn poll(&mut self) {
        if let Some(ref rx) = self.rx {
            if let Ok(result) = rx.try_recv() {
                self.rx = None;
                match result {
                    Ok(articles) => {
                        self.articles = articles;
                        self.loading = false;
                    }
                    Err(err) => {
                        self.error = Some(err);
                        self.loading = false;
                    }
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> ViewEvent {
        match key.code {
            KeyCode::Char('w') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Char('s') | KeyCode::Down => {
                self.scroll += 1;
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.scroll += 10;
            }
            _ => {}
        }
        ViewEvent::None
    }
}

/// Frontier's CMS, which the GalNet pages on elitedangerous.com are built from (JSON:API): the
/// newest articles first. The old page, community.elitedangerous.com/en/galnet, now redirects to
/// a new site whose list holds teasers only (2026-10-29).
const GALNET_URL: &str = "https://cms.zaonce.net/en-GB/jsonapi/node/galnet_article?sort=-published_at&page%5Blimit%5D=20";

#[derive(serde::Deserialize)]
struct GalnetPage {
    data: Vec<GalnetNode>,
}

#[derive(serde::Deserialize)]
struct GalnetNode {
    attributes: GalnetArticle,
}

#[derive(serde::Deserialize)]
struct GalnetArticle {
    title: String,
    /// The in-game date, as "22 OCT 3312".
    field_galnet_date: Option<String>,
    body: Option<GalnetBody>,
}

#[derive(serde::Deserialize)]
struct GalnetBody {
    /// Plain text, one paragraph per line.
    value: String,
}

async fn fetch_articles_async(client: reqwest::Client) -> Result<Vec<Article>, String> {
    let page: GalnetPage = client
        .get(GALNET_URL)
        .header("Accept", "application/vnd.api+json")
        // Without a User-Agent the CMS answers 403.
        .header("User-Agent", concat!("edcas-client/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| { tracing::error!("Couldn't fetch Galnet: {}", e); format!("Couldn't fetch Galnet: {}", e) })?
        .json()
        .await
        .map_err(|e| { tracing::error!("Couldn't read Galnet's answer: {}", e); format!("Couldn't read Galnet's answer: {}", e) })?;
    let articles = articles_of(page);
    tracing::debug!("Success fetching galnet {} articles", articles.len());
    Ok(articles)
}

fn articles_of(page: GalnetPage) -> Vec<Article> {
    page.data
        .into_iter()
        .map(|node| {
            let a = node.attributes;
            Article { title: a.title, date: a.field_galnet_date.unwrap_or_default(), text: a.body.map(|b| b.value).unwrap_or_default() }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn galnet_articles_come_from_the_cms_answer() {
        let json = r#"{"jsonapi":{"version":"1.1"},"data":[
            {"type":"node--galnet_article","id":"e0b7","attributes":{"title":"Anaconda Mk II Field Trials",
             "field_galnet_date":"22 OCT 3312","body":{"value":"First.\r\nSecond.","format":null},"field_slug":"x"}},
            {"type":"node--galnet_article","id":"e0b8","attributes":{"title":"No body","field_galnet_date":null,"body":null}}],
            "links":{}}"#;
        let articles = articles_of(serde_json::from_str(json).unwrap());
        assert_eq!(articles.len(), 2);
        assert_eq!((articles[0].title.as_str(), articles[0].date.as_str()), ("Anaconda Mk II Field Trials", "22 OCT 3312"));
        assert_eq!(articles[0].text, "First.\r\nSecond.");
        assert_eq!((articles[1].date.as_str(), articles[1].text.as_str()), ("", ""));
    }
}
