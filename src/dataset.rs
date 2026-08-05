use crate::tokenizer::Tokenizer;
use regex::Regex;
use scraper::{Html, Selector};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RawDocument {
    pub source: String,
    pub title: String,
    pub content: String,
}

pub struct WebScraper;

impl WebScraper {
    pub fn scrape_url(url_str: &str) -> Result<RawDocument, Box<dyn std::error::Error>> {
        let full_url = if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
            format!("https://{}", url_str)
        } else {
            url_str.to_string()
        };

        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let response = client.get(&full_url).send()?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response.text()?;

        if content_type.contains("text/plain") {
            let title = full_url.split('/').last().unwrap_or(&full_url).to_string();
            return Ok(RawDocument {
                source: full_url,
                title,
                content: body.trim().to_string(),
            });
        }

        let document = Html::parse_document(&body);

        let title_sel = Selector::parse("title").unwrap();
        let title = document
            .select(&title_sel)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| full_url.clone());

        let p_sel = Selector::parse("h1, h2, h3, h4, p, li, blockquote, pre, code").unwrap();
        let mut paragraphs = Vec::new();

        for element in document.select(&p_sel) {
            let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if text.len() > 5 {
                paragraphs.push(text);
            }
        }

        let raw_text = if paragraphs.is_empty() {
            let body_sel = Selector::parse("body").unwrap();
            document
                .select(&body_sel)
                .next()
                .map(|e| e.text().collect::<Vec<_>>().join(" "))
                .unwrap_or(body)
        } else {
            paragraphs.join("\n\n")
        };

        let space_re = Regex::new(r"[ \t]{2,}")?;
        let newline_re = Regex::new(r"\n{3,}")?;

        let cleaned = space_re.replace_all(&raw_text, " ");
        let cleaned = newline_re.replace_all(&cleaned, "\n\n");

        Ok(RawDocument {
            source: full_url,
            title: title.trim().to_string(),
            content: cleaned.trim().to_string(),
        })
    }
}

pub struct DocumentReader;

impl DocumentReader {
    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<RawDocument, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        Ok(RawDocument {
            source: path.to_string_lossy().to_string(),
            title,
            content: content.trim().to_string(),
        })
    }

    pub fn read_directory<P: AsRef<Path>>(dir: P, recursive: bool) -> Vec<RawDocument> {
        let mut docs = Vec::new();
        let supported_exts = ["txt", "md", "csv", "json", "log", "rst", "py", "rs", "js", "html"];

        let walk_dir = |p: &Path, docs: &mut Vec<RawDocument>, recurse: bool| {
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                            if supported_exts.contains(&ext.to_lowercase().as_str()) {
                                if let Ok(doc) = DocumentReader::read_file(&entry_path) {
                                    if !doc.content.is_empty() {
                                        docs.push(doc);
                                    }
                                }
                            }
                        }
                    } else if entry_path.is_dir() && recurse {
                        let name = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if !name.starts_with('.') && name != "target" && name != "node_modules" {
                            let _ = DocumentReader::read_directory(&entry_path, true);
                        }
                    }
                }
            }
        };

        walk_dir(dir.as_ref(), &mut docs, recursive);
        docs
    }
}

pub struct TextChunker;

impl TextChunker {
    pub fn chunk_text(
        text: &str,
        tokenizer: &Tokenizer,
        chunk_size: usize,
        _overlap: usize,
    ) -> Vec<(String, Vec<usize>)> {
        let mut chunks = Vec::new();
        let paragraphs: Vec<&str> = text.split("\n\n").map(|p| p.trim()).filter(|p| !p.is_empty()).collect();

        let mut current_text = String::new();
        let mut current_tokens = Vec::new();

        for para in paragraphs {
            let para_tokens = tokenizer.encode(para, false);

            if current_tokens.len() + para_tokens.len() <= chunk_size {
                if !current_text.is_empty() {
                    current_text.push_str("\n\n");
                }
                current_text.push_str(para);
                current_tokens.extend(para_tokens);
            } else {
                if !current_tokens.is_empty() {
                    chunks.push((current_text.clone(), current_tokens.clone()));
                }

                current_text = para.to_string();
                current_tokens = para_tokens;
            }
        }

        if !current_tokens.is_empty() {
            chunks.push((current_text, current_tokens));
        }

        chunks
    }
}

