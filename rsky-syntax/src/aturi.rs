use anyhow::{bail, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt::Display;
use url::Url;

pub fn atp_uri_regex(input: &str) -> Option<Vec<&str>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?i)^(at://)?((?:did:[a-z0-9:%-]+)|(?:[a-z0-9][a-z0-9.:-]*))(/[^?#\s]*)?(\?[^#\s]+)?(#[^\s]+)?$").unwrap();
    }
    if let Some(captures) = RE.captures(input) {
        Some(
            captures
                .iter()
                .skip(1) // Skip the first capture which is the entire match
                .map(|c| c.map_or("", |m| m.as_str()))
                .collect(),
        )
    } else {
        None
    }
}

pub fn relative_regex(input: &str) -> Option<Vec<&str>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?i)^(/[^?#\s]*)?(\?[^#\s]+)?(#[^\s]+)?$").unwrap();
    }
    if let Some(captures) = RE.captures(input) {
        Some(
            captures
                .iter()
                .skip(1) // Skip the first capture which is the entire match
                .map(|c| c.map_or("", |m| m.as_str()))
                .collect(),
        )
    } else {
        None
    }
}

pub struct ParsedOutput {
    pub hash: String,
    pub host: String,
    pub pathname: String,
    pub search_params: Vec<(String, String)>,
}

pub struct ParsedRelativeOutput {
    pub hash: String,
    pub pathname: String,
    pub search_params: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AtUri {
    pub hash: String,
    pub host: String,
    pub pathname: String,
    pub search_params: Vec<(String, String)>,
}

impl AtUri {
    pub fn new(uri: String, base: Option<String>) -> Result<Self> {
        let parsed: ParsedOutput = match base {
            Some(base) => match parse(&base)? {
                None => bail!("Invalid at uri: `{base}`"),
                Some(parsed_base) => match parse_relative(&uri)? {
                    None => bail!("Invalid path: `{uri}`"),
                    Some(relativep) => ParsedOutput {
                        hash: relativep.hash,
                        host: parsed_base.host,
                        pathname: relativep.pathname,
                        search_params: relativep.search_params,
                    },
                },
            },
            None => match parse(&uri)? {
                None => bail!("Invalid at uri: `{uri}`"),
                Some(result) => result,
            },
        };
        Ok(Self {
            hash: parsed.hash,
            host: parsed.host,
            pathname: parsed.pathname,
            search_params: parsed.search_params,
        })
    }

    pub fn make(
        handle_or_did: String,
        collection: Option<String>,
        rkey: Option<String>,
    ) -> Result<Self> {
        let mut str = handle_or_did;
        if let Some(collection) = collection {
            str += format!("/{collection}").as_str();
        }
        if let Some(rkey) = rkey {
            str += format!("/{rkey}").as_str();
        }
        AtUri::new(str, None)
    }

    pub fn get_protocol(&self) -> String {
        "at:".to_string()
    }

    pub fn get_origin(&self) -> String {
        format!("at://{}", self.host)
    }

    pub fn get_hostname(&self) -> &String {
        &self.host
    }

    pub fn set_hostname(&mut self, v: String) -> () {
        self.host = v;
    }

    pub fn get_search(&self) -> Result<Option<String>> {
        let url = Url::parse_with_params("http://example.com", &self.search_params)?;
        match url.query() {
            Some(query) => Ok(Some(query.to_string())),
            None => Ok(None),
        }
    }

    pub fn set_search(&mut self, v: String) -> Result<()> {
        let dummy_url = format!("http://example.com{}", v);
        let url = Url::parse(&dummy_url)?;
        let query_pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|pair| (pair.0.to_string(), pair.1.to_string()))
            .collect();
        self.search_params = query_pairs;
        Ok(())
    }

    pub fn get_collection(&self) -> String {
        match &self.pathname.split("/").collect::<Vec<&str>>().get(1) {
            Some(collection) => collection.to_string(),
            None => "".to_string(),
        }
    }

    pub fn set_collection(&mut self, v: String) -> () {
        let mut parts: Vec<String> = self
            .pathname
            .split("/")
            .collect::<Vec<&str>>()
            .into_iter()
            .map(|p| p.to_string())
            .collect::<Vec<String>>();
        if parts.len() > 0 {
            parts[0] = v;
        } else {
            parts.push(v);
        }
        self.pathname = parts.join("/");
    }

    pub fn get_rkey(&self) -> String {
        match &self.pathname.split("/").collect::<Vec<&str>>().get(2) {
            Some(rkey) => rkey.to_string(),
            None => "".to_string(),
        }
    }

    pub fn set_rkey(&mut self, v: String) -> () {
        let mut parts: Vec<String> = self
            .pathname
            .split("/")
            .collect::<Vec<&str>>()
            .into_iter()
            .map(|p| p.to_string())
            .collect::<Vec<String>>();
        if parts.len() > 1 {
            parts[1] = v;
        } else if parts.len() > 0 {
            parts.push(v);
        } else {
            parts.push("undefined".to_string());
            parts.push(v);
        }
        self.pathname = parts.join("/");
    }

    pub fn get_href(&self) -> String {
        self.to_string()
    }
}

pub fn parse(str: &String) -> Result<Option<ParsedOutput>> {
    match atp_uri_regex(str) {
        None => Ok(None),
        Some(matches) => {
            // The query string we want to parse
            // e.g. `?q=URLUtils.searchParams&topic=api`
            let query_string = matches[3];
            // Create a dummy base URL and append the query string
            let dummy_url = format!("http://example.com{}", query_string);
            // Parse the URL
            let url = Url::parse(&dummy_url)?;
            let query_pairs: Vec<(String, String)> = url
                .query_pairs()
                .map(|pair| (pair.0.to_string(), pair.1.to_string()))
                .collect();
            Ok(Some(ParsedOutput {
                hash: matches[4].to_string(),
                host: matches[1].to_string(),
                pathname: matches[2].to_string(),
                search_params: query_pairs,
            }))
        }
    }
}

pub fn parse_relative(str: &String) -> Result<Option<ParsedRelativeOutput>> {
    match relative_regex(str) {
        None => Ok(None),
        Some(matches) => {
            // The query string we want to parse
            // e.g. `?q=URLUtils.searchParams&topic=api`
            let query_string = matches[1];
            // Create a dummy base URL and append the query string
            let dummy_url = format!("http://example.com{}", query_string);
            // Parse the URL
            let url = Url::parse(&dummy_url)?;
            let query_pairs: Vec<(String, String)> = url
                .query_pairs()
                .map(|pair| (pair.0.to_string(), pair.1.to_string()))
                .collect();
            Ok(Some(ParsedRelativeOutput {
                hash: matches[2].to_string(),
                pathname: matches[0].to_string(),
                search_params: query_pairs,
            }))
        }
    }
}

impl Display for AtUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut path = match self.pathname == "" {
            true => "/".to_string(),
            false => self.pathname.clone(),
        };
        if !path.starts_with("/") {
            path = format!("/{path}");
        }
        let qs = match self.get_search() {
            Ok(Some(search_params)) if !search_params.starts_with("?") && search_params != "" => {
                format!("?{search_params}")
            }
            Ok(Some(search_params)) => search_params,
            _ => "".to_string(),
        };
        let hash = match self.hash == "" {
            true => self.hash.clone(),
            false => format!("#{}", self.hash),
        };
        write!(f, "at://{}{}{}{}", self.host, path, qs, hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function that constructs AtUri using the new() method and returns a Result
    fn create_at_uri(
        host: &str,
        pathname: &str,
        search_params: Vec<(String, String)>,
        hash: &str,
    ) -> Result<AtUri> {
        let mut uri = AtUri::new(host.to_string(), None)?;
        uri.pathname = pathname.to_string();
        uri.search_params = search_params;
        uri.hash = hash.to_string();
        Ok(uri)
    }

    #[test]
    fn test_display_basic_uri() {
        // Test a basic AT URI with just host and pathname
        let uri = create_at_uri("example.com", "app.bsky.feed.post/123", vec![], "")
            .expect("Should create valid basic URI");
        assert_eq!(uri.to_string(), "at://example.com/app.bsky.feed.post/123");
    }

    #[test]
    fn test_display_empty_pathname() {
        // Empty pathname should result in a single slash
        let uri = create_at_uri("example.com", "", vec![], "")
            .expect("Should create valid URI with empty pathname");
        assert_eq!(uri.to_string(), "at://example.com/");
    }

    #[test]
    fn test_display_with_did() {
        // Test URI with DID in host
        let uri = create_at_uri(
            "did:plc:44ybard66vv44zksje25o7dz",
            "app.bsky.feed.post/123",
            vec![],
            "",
        )
        .expect("Should create valid URI with DID");
        assert_eq!(
            uri.to_string(),
            "at://did:plc:44ybard66vv44zksje25o7dz/app.bsky.feed.post/123"
        );
    }

    #[test]
    fn test_display_with_search_params() {
        // Test URI with query parameters
        let uri = create_at_uri(
            "example.com",
            "app.bsky.feed.post/123",
            vec![
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
            ],
            "",
        )
        .expect("Should create valid URI with search parameters");
        assert_eq!(
            uri.to_string(),
            "at://example.com/app.bsky.feed.post/123?key1=value1&key2=value2"
        );
    }

    #[test]
    fn test_display_with_hash() {
        // Test URI with hash fragment
        let uri = create_at_uri("example.com", "app.bsky.feed.post/123", vec![], "fragment")
            .expect("Should create valid URI with hash fragment");
        assert_eq!(
            uri.to_string(),
            "at://example.com/app.bsky.feed.post/123#fragment"
        );
    }

    #[test]
    fn test_display_complete_uri() {
        // Test URI with all components
        let uri = create_at_uri(
            "example.com",
            "app.bsky.feed.post/123",
            vec![("key".to_string(), "value".to_string())],
            "fragment",
        )
        .expect("Should create valid complete URI");
        assert_eq!(
            uri.to_string(),
            "at://example.com/app.bsky.feed.post/123?key=value#fragment"
        );
    }

    #[test]
    fn test_display_pathname_formatting() {
        // Test pathname without leading slash gets one added
        let uri = create_at_uri("example.com", "app.bsky.feed.post/123", vec![], "")
            .expect("Should create valid URI with pathname");
        assert!(uri.to_string().contains("/app.bsky.feed.post/123"));
    }

    #[test]
    fn test_display_search_params_formatting() {
        // Test search params formatting without leading question mark
        let uri = create_at_uri(
            "example.com",
            "path",
            vec![("key".to_string(), "value".to_string())],
            "",
        )
        .expect("Should create valid URI with search params");
        let result = uri.to_string();
        assert!(result.contains("?key=value"));
        assert!(
            !result.contains("??"),
            "Should not contain double question marks"
        );
    }

    #[test]
    fn test_display_hash_formatting() {
        // Test hash formatting without leading hash symbol
        let uri = create_at_uri("example.com", "path", vec![], "fragment")
            .expect("Should create valid URI with hash");
        let result = uri.to_string();
        assert!(result.contains("#fragment"));
        assert!(
            !result.contains("##"),
            "Should not contain double hash symbols"
        );
    }

    #[test]
    fn test_display_spec_compliance() {
        // Test various cases from the AT URI spec examples
        let cases = vec![
            (
                "foo.com",
                "com.example.foo/123",
                "at://foo.com/com.example.foo/123",
            ),
            (
                "did:plc:44ybard66vv44zksje25o7dz",
                "app.bsky.feed.post/3jwdwj2ctlk26",
                "at://did:plc:44ybard66vv44zksje25o7dz/app.bsky.feed.post/3jwdwj2ctlk26",
            ),
            (
                "bnewbold.bsky.team",
                "app.bsky.feed.post/3jwdwj2ctlk26",
                "at://bnewbold.bsky.team/app.bsky.feed.post/3jwdwj2ctlk26",
            ),
        ];

        for (host, pathname, expected) in cases {
            let uri = create_at_uri(host, pathname, vec![], "")
                .expect("Should create valid URI for spec compliance test");
            assert_eq!(uri.to_string(), expected);
        }
    }

    // one negative test
    #[test]
    fn test_invalid_query_parameters() {
        let invalid_cases = vec![
            // Empty key
            vec![("".to_string(), "value".to_string())],
            // Multiple question marks
            vec![("??key".to_string(), "value".to_string())],
            // Invalid characters in query params
            vec![("key#invalid".to_string(), "value".to_string())],
        ];

        for search_params in invalid_cases {
            let result = create_at_uri("example.com", "path", search_params.clone(), "");

            // If construction succeeds (query param validation might not be in new())
            if let Ok(uri) = result {
                let display_result = uri.to_string();
                assert!(
                    display_result.matches('?').count() <= 1,
                    "Query string should contain at most one question mark: {}",
                    display_result
                );
            }
        }
    }
}
