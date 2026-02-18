use serde::{Deserialize, Deserializer};
use std::{fs, io::BufReader, path::Path};
use url::Url;

impl Har {
    pub async fn from_file(path: &Path) -> anyhow::Result<Self> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let har = serde_json::from_reader(reader)?;

        Ok(har)
    }
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Url::parse(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Har {
    pub log: Log,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    pub version: Option<String>,
    pub creator: Option<Creator>,
    pub browser: Option<Browser>,
    pub pages: Option<Vec<Page>>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Browser {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    pub page_timings: PageTimings,
    pub started_date_time: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageTimings {
    pub on_content_load: Option<f64>,
    pub on_load: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub started_date_time: String,
    pub request: Request,
    pub response: Response,
    pub cache: Cache,
    pub timings: Timings,
    pub time: f64,
    #[serde(rename = "_securityState")]
    pub security_state: Option<String>,
    pub pageref: Option<String>,
    #[serde(rename = "serverIPAddress")]
    pub server_ipaddress: Option<String>,
    pub connection: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub body_size: Option<i64>,
    pub method: String,
    #[serde(deserialize_with = "deserialize_url")]
    pub url: url::Url,
    pub http_version: String,
    pub headers: Vec<Header>,
    pub cookies: Vec<Cookie>,
    pub query_string: Vec<QueryString>,
    pub headers_size: Option<i64>,
    pub post_data: Option<PostData>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryString {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostData {
    pub mime_type: String,
    pub params: Option<Vec<Param>>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Param {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub status: i64,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<Header>,
    pub cookies: Vec<Cookie>,
    pub content: Content,
    #[serde(rename = "redirectURL")]
    pub redirect_url: String,
    pub headers_size: Option<i64>,
    pub body_size: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub mime_type: Option<String>,
    pub size: Option<i64>,
    pub text: Option<String>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cache {}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timings {
    pub blocked: Option<f64>,
    pub dns: Option<f64>,
    pub ssl: Option<f64>,
    pub connect: Option<f64>,
    pub send: Option<f64>,
    pub wait: Option<f64>,
    pub receive: Option<f64>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_HAR: &str = r#"{
        "log": {
            "version": "1.2",
            "creator": { "name": "test-tool", "version": "1.0" },
            "entries": [
                {
                    "startedDateTime": "2024-06-01T10:00:00.000Z",
                    "time": 123.4,
                    "request": {
                        "method": "GET",
                        "url": "https://example.com/api/data?q=hello",
                        "httpVersion": "HTTP/1.1",
                        "headers": [
                            { "name": "Accept", "value": "application/json" }
                        ],
                        "cookies": [],
                        "queryString": [
                            { "name": "q", "value": "hello" }
                        ],
                        "headersSize": -1,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "headers": [
                            { "name": "Content-Type", "value": "application/json" }
                        ],
                        "cookies": [],
                        "content": {
                            "size": 27,
                            "mimeType": "application/json",
                            "text": "{\"status\":\"ok\",\"count\":42}"
                        },
                        "redirectURL": "",
                        "headersSize": -1,
                        "bodySize": 27
                    },
                    "cache": {},
                    "timings": {
                        "send": 1.0,
                        "wait": 120.0,
                        "receive": 2.4
                    }
                }
            ]
        }
    }"#;

    #[test]
    fn parse_minimal_har() {
        let har: Har = serde_json::from_str(MINIMAL_HAR).expect("should parse");
        assert_eq!(har.log.entries.len(), 1);
    }

    #[test]
    fn parse_entry_request_fields() {
        let har: Har = serde_json::from_str(MINIMAL_HAR).unwrap();
        let req = &har.log.entries[0].request;
        assert_eq!(req.method, "GET");
        assert_eq!(req.url.host_str(), Some("example.com"));
        assert_eq!(req.url.path(), "/api/data");
        assert_eq!(req.query_string.len(), 1);
        assert_eq!(req.query_string[0].name, "q");
        assert_eq!(req.query_string[0].value, "hello");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].name, "Accept");
    }

    #[test]
    fn parse_entry_response_fields() {
        let har: Har = serde_json::from_str(MINIMAL_HAR).unwrap();
        let resp = &har.log.entries[0].response;
        assert_eq!(resp.status, 200);
        assert_eq!(resp.status_text, "OK");
        assert_eq!(resp.headers[0].value, "application/json");
        assert_eq!(resp.content.size, Some(27));
        assert_eq!(resp.content.text.as_deref(), Some("{\"status\":\"ok\",\"count\":42}"));
        assert!(resp.content.encoding.is_none());
    }

    #[test]
    fn parse_entry_timing_and_duration() {
        let har: Har = serde_json::from_str(MINIMAL_HAR).unwrap();
        let entry = &har.log.entries[0];
        assert_eq!(entry.time, 123.4);
        assert_eq!(entry.timings.send, Some(1.0));
        assert_eq!(entry.timings.wait, Some(120.0));
        assert_eq!(entry.timings.blocked, None);
    }

    #[test]
    fn parse_creator_metadata() {
        let har: Har = serde_json::from_str(MINIMAL_HAR).unwrap();
        let creator = har.log.creator.unwrap();
        assert_eq!(creator.name.as_deref(), Some("test-tool"));
        assert_eq!(creator.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn parse_log_version() {
        let har: Har = serde_json::from_str(MINIMAL_HAR).unwrap();
        assert_eq!(har.log.version.as_deref(), Some("1.2"));
    }

    #[test]
    fn parse_url_components() {
        let har: Har = serde_json::from_str(MINIMAL_HAR).unwrap();
        let url = &har.log.entries[0].request.url;
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
        assert_eq!(url.path(), "/api/data");
        assert!(url.as_str().contains("q=hello"));
    }

    #[test]
    fn reject_invalid_url() {
        let bad = MINIMAL_HAR.replace(
            "\"url\": \"https://example.com/api/data?q=hello\"",
            "\"url\": \"not a url at all!!!\"",
        );
        let result: Result<Har, _> = serde_json::from_str(&bad);
        assert!(result.is_err(), "invalid URL should fail to deserialize");
    }

    #[test]
    fn parse_empty_entries_list() {
        let json = r#"{
            "log": {
                "entries": []
            }
        }"#;
        let har: Har = serde_json::from_str(json).expect("empty entries should parse");
        assert_eq!(har.log.entries.len(), 0);
    }

    #[test]
    fn parse_base64_encoded_response_body() {
        use base64::prelude::*;
        let body = r#"{"secret":"value"}"#;
        let encoded = BASE64_STANDARD.encode(body);
        let json = MINIMAL_HAR.replace(
            "{\"status\":\"ok\",\"count\":42}",
            &encoded,
        ).replace(
            "\"size\": 27",
            &format!("\"size\": {}, \"encoding\": \"base64\"", encoded.len()),
        );
        let har: Har = serde_json::from_str(&json).expect("base64 entry should parse");
        assert_eq!(
            har.log.entries[0].response.content.encoding.as_deref(),
            Some("base64")
        );
    }
}
