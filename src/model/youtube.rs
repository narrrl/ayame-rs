use chrono::{DateTime, Utc};
use html_escape;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::string::ToString;
use strum_macros::Display;
use tracing::info;
use url::Url;

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct YoutubeResult {
    kind: String,
    etag: String,
    id: Id,
    snippet: Snippet,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Id {
    kind: String,
    #[serde(rename = "videoId", alias = "channelId", alias = "playlistId")]
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Snippet {
    #[serde(rename = "publishedAt")]
    published_at: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    title: String,
    description: String,
    thumbnails: Thumbnails,
    #[serde(rename = "channelTitle")]
    channel_title: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Thumbnails {
    default: Thumbnail,
    medium: Thumbnail,
    high: Thumbnail,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thumbnail {
    url: String,
    width: Option<i32>,
    height: Option<i32>,
}
impl Thumbnail {
    #[allow(dead_code)]
    pub fn url(&self) -> String {
        self.url.clone()
    }
    #[allow(dead_code)]
    pub fn width(&self) -> Option<i32> {
        self.width
    }
    #[allow(dead_code)]
    pub fn height(&self) -> Option<i32> {
        self.height
    }
}
impl YoutubeResult {
    #[allow(dead_code)]
    pub fn url(&self) -> String {
        match self.result_type() {
            Type::CHANNEL => format!("https://www.youtube.com/channel/{}", self.id.id),
            Type::VIDEO => format!("https://www.youtube.com/watch?v={}", self.id.id),
            Type::PLAYLIST => format!("https://www.youtube.com/watch?list={}", self.id.id),
            Type::NONE => format!("{}", Type::NONE),
        }
    }

    #[allow(dead_code)]
    pub fn title(&self) -> String {
        html_escape::decode_html_entities(&self.snippet.title).to_string()
    }

    #[allow(dead_code)]
    pub fn channel_url(&self) -> String {
        format!(
            "https://www.youtube.com/channel/{}",
            self.snippet.channel_id
        )
    }

    #[allow(dead_code)]
    pub fn channel_name(&self) -> String {
        html_escape::decode_html_entities(&self.snippet.channel_title).to_string()
    }

    #[allow(dead_code)]
    pub fn small_thumbnail(&self) -> Thumbnail {
        self.snippet.thumbnails.default.clone()
    }

    #[allow(dead_code)]
    pub fn medium_thumbnail(&self) -> Thumbnail {
        self.snippet.thumbnails.medium.clone()
    }
    #[allow(dead_code)]
    pub fn thumbnail(&self) -> Thumbnail {
        self.snippet.thumbnails.high.clone()
    }

    #[allow(dead_code)]
    pub fn time_published(&self) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&self.snippet.published_at)
            .expect("Couldn't convert YouTube-Video publish time")
            .into()
    }

    #[allow(dead_code)]
    pub fn result_type(&self) -> Type {
        match self.id.kind.as_ref() {
            "youtube#channel" => Type::CHANNEL,
            "youtube#video" => Type::VIDEO,
            "youtube#playlist" => Type::PLAYLIST,
            _ => Type::NONE,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YoutubeResponse {
    items: Vec<YoutubeResult>,
}

#[allow(dead_code)]
impl<'a> YoutubeResponse {
    pub fn results(&'a self) -> &'a Vec<YoutubeResult> {
        &self.items
    }
}
#[derive(Display, Debug)]
#[strum(serialize_all = "lowercase")]
#[allow(dead_code)]
pub enum Type {
    VIDEO,
    CHANNEL,
    PLAYLIST,
    NONE,
}

#[allow(dead_code)]
pub struct YoutubeSearch {
    api_key: String,
    amount: Option<u8>,
    result_type: Type,
    queries: Vec<(String, String)>,
}

impl YoutubeSearch {
    #[allow(dead_code)]
    pub fn new(api_key: &str) -> YoutubeSearch {
        YoutubeSearch {
            api_key: api_key.to_string(),
            amount: None,
            result_type: Type::NONE,
            queries: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn set_amount<'a>(&'a mut self, amount: u8) -> &'a mut YoutubeSearch {
        self.amount = Some(amount);
        self
    }

    #[allow(dead_code)]
    pub fn set_filter<'a>(&'a mut self, result_type: Type) -> &'a mut YoutubeSearch {
        self.result_type = result_type;
        self
    }

    #[allow(dead_code)]
    pub fn add_query<'a>(&'a mut self, query: &str, data: &str) -> &'a mut Self {
        self.queries.push((String::from(query), String::from(data)));
        self
    }

    #[allow(dead_code)]
    fn _build_query(&self, search_term: &str) -> String {
        let mut query = String::new();
        query.push_str("part=snippet");
        query.push_str(&format!("&q={}", search_term));
        if let Some(amount) = self.amount {
            query.push_str(&format!("&maxResults={}", amount));
        }
        match self.result_type {
            Type::NONE => (),
            _ => query.push_str(&format!("&type={}", self.result_type)),
        };
        query.push_str(&format!("&key={}", self.api_key));

        for (q, d) in self.queries.iter() {
            query.push_str(&format!("&{}={}", q, d));
        }

        query
    }

    #[allow(dead_code)]
    pub async fn search(&self, query: &str) -> std::result::Result<YoutubeResponse, Error> {
        let mut url = Url::parse("https://www.googleapis.com/youtube/v3/search").unwrap();
        let query = self._build_query(query);
        url.set_query(Some(&query));
        let url_string = url.to_string();
        info!("created url {}", &url_string);

        let res = match reqwest::get(&url_string).await {
            Ok(res) => res,
            Err(why) => {
                return Err(Error::from(format!(
                    "youtube search failed to send request: {:?}",
                    why
                )));
            }
        };

        let status = res.status();

        let body = match res.text().await {
            Ok(body) => body,
            Err(why) => {
                return Err(Error::from(format!(
                    "youtube search failed to parse body: {:?}",
                    why
                )));
            }
        };
        if !status.eq(&StatusCode::OK) {
            return Err(Error::from(format!(
                "youtube search failed with wrong status code: {:?}",
                status
            )));
        }

        let res: YoutubeResponse = match serde_json::from_str(&body) {
            Ok(res) => res,
            Err(why) => {
                return Err(Error::from(format!(
                    "youtube search failed to parse from json: {:?}",
                    why
                )));
            }
        };
        Ok(res)
    }
}

#[allow(dead_code)]
pub fn hyperlink_result(result: &YoutubeResult) -> String {
    if let Type::CHANNEL = result.result_type() {
        return format!("[{}]({})", result.title(), result.url(),);
    }
    format!(
        "[{} - {}]({})",
        result.title(),
        result.channel_name(),
        result.url()
    )
}

#[cfg(test)]
mod tests {
    use crate::model::youtube::*;

    #[tokio::test]
    // github run fails because no config.toml is provided
    #[ignore]
    async fn test_search() -> Result<(), Box<dyn std::error::Error>> {
        let config = &crate::CONFIG;
        let mut req = YoutubeSearch::new(&config.youtube_api_key());
        req.set_filter(Type::CHANNEL).set_amount(5);

        let res = req.search("noel ch").await?;
        println!("Searching for {}...", "noel ch");
        for result in res.results().iter() {
            println!(
                "Found {}: {} {}",
                result.result_type(),
                result.title(),
                result.url()
            );
        }
        Ok(())
    }

    #[tokio::test]
    // github run fails because no config.toml is provided
    #[ignore]
    async fn test_search_video() -> Result<(), Box<dyn std::error::Error>> {
        let config = &crate::CONFIG;
        let mut req = YoutubeSearch::new(&config.youtube_api_key());
        req.set_filter(Type::VIDEO).set_amount(5);

        println!("Searching for {}...", "lofi stream");
        let res = req.search("lofi stream").await?;
        for result in res.results().iter() {
            println!(
                "Found {}: {} {}",
                result.result_type(),
                result.title(),
                result.url()
            );
        }
        Ok(())
    }
}
