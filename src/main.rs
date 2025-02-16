use std::env;
use std::fs;
use std::thread::sleep;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use scraper::Html;
use scraper::Selector;
use regex::Regex;
use url::Url;
use dotenv::dotenv;
use reqwest;
use unicode_normalization::UnicodeNormalization;


const QUOTA_RESET_WAIT: u64 = 5; // 配额重置等待时间（秒）
const MAX_RETRIES: usize = 3;
const REQUEST_INTERVAL: f64 = 3.0; // 增加到3秒
const TAG_BLACKLIST: [&str; 3] = ["其他", "未知", "未分类"];

#[derive(Debug, Clone)]
struct Bookmark {
    url: String,
    title: String,
    tags: Option<Vec<String>>,
    date_added: Option<DateTime<Utc>>,
}

struct AiResponse {
    title: String,
    tags: Vec<String>,
}

#[derive(Debug)]
struct Stats {
    total: usize,
    updated: usize,
    failed: usize,
}

// 添加标题处理函数
fn clean_title(title: &str) -> String {
    // 移除多余空白字符
    let cleaned = title
        .trim()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");

    // 标准化 Unicode 字符
    cleaned.nfkc().collect::<String>()
}

// 保存进度
fn save_progress(bookmarks: &[Bookmark], filename: &str) -> std::io::Result<()> {
    let output_html = generate_bookmarks_html(bookmarks);
    fs::write(filename, output_html)?;
    println!("进度已保存至：{}", filename);
    Ok(())
}

fn parse_bookmarks_html(html_content: &str) -> Vec<Bookmark> {
    let document = Html::parse_document(html_content);
    let selector = Selector::parse("a").unwrap();

    document
        .select(&selector)
        .filter_map(|element| {
            let url = element.value().attr("href")?;

            // 获取并清理标题
            let raw_title = element.text().collect::<String>();
            let title = if raw_title.trim().is_empty() {
                // 如果标题为空，尝试从 URL 生成标题
                Url::parse(url)
                    .ok()
                    .and_then(|parsed_url| {
                        parsed_url.path_segments()
                            .and_then(|segments| segments.last())
                            .map(|last_segment| last_segment.replace('-', " "))
                    })
                    .unwrap_or_else(|| "无标题".to_string())
            } else {
                clean_title(&raw_title)
            };

            let date_added = element.value().attr("add_date")
                .and_then(|date_str| date_str.parse::<i64>().ok())
                .map(|timestamp| {
                    DateTime::from_timestamp(timestamp, 0)
                        .unwrap_or_else(|| Utc::now())
                });

            Some(Bookmark {
                url: url.to_string(),
                title,
                tags: None,
                date_added,
            })
        })
        .collect()
}


fn generate_bookmarks_html(bookmarks: &[Bookmark]) -> String {
    let mut html = String::from(
        "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
         <META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n\
         <META NAME=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <TITLE>Bookmarks</TITLE>\n\
         <H1>Bookmarks</H1>\n\
         <DL><p>\n"
    );

    for bookmark in bookmarks {
        let tags_str = bookmark.tags
            .as_ref()
            .map(|tags| tags.join(", "))
            .unwrap_or_default();

        let date_str = bookmark.date_added
            .map(|date| date.timestamp().to_string())
            .unwrap_or_default();

        // 添加更多属性以支持搜索
        html.push_str(&format!(
            "    <DT><A HREF=\"{}\" \
             ADD_DATE=\"{}\" \
             TAGS=\"{}\" \
             TITLE=\"{}\" \
             SEARCH_TERMS=\"{}, {}\">{}</A>\n",
            bookmark.url,
            date_str,
            tags_str,
            bookmark.title,
            bookmark.title,
            tags_str,
            bookmark.title
        ));
    }

    html.push_str("</DL><p>");
    html
}

fn validate_url(url: &str) -> bool {
    if let Ok(parsed_url) = Url::parse(url) {
        let scheme = parsed_url.scheme();
        return (scheme == "http" || scheme == "https") && !parsed_url.host_str().unwrap_or("").is_empty();
    }
    false
}

async fn get_tags_from_gemini(url: &str, api_key: &str) -> Option<AiResponse> {
    println!("开始处理URL: {}", url);
    let client = reqwest::Client::new();

    let prompt = format!(
        "请根据以下网址内容生成标题和标签：\n\
         1. 生成一个简洁准确的中文标题（15-30字）\n\
         2. 生成5个精准标签，使用逗号分隔\n\
         3. 标签要求：中文名词短语，最长15字，排除通用词汇\n\
         4. 返回格式：\n\
         标题：网页标题\n\
         标签：标签1,标签2,标签3\n\n\
         当前网址：{}",
        url
    );

    let api_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let request_body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": prompt
            }]
        }],
        "generationConfig": {
            "temperature": 0.7,
            "topK": 40,
            "topP": 0.95,
            "maxOutputTokens": 1024,
        }
    });

    for attempt in 0..MAX_RETRIES {
        println!("尝试调用 API (attempt {}/{})", attempt + 1, MAX_RETRIES);

        match client.post(&api_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await {
            Ok(response) => {
                if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    println!("达到 API 配额限制，等待 {} 秒后重试...", QUOTA_RESET_WAIT);
                    sleep(Duration::from_secs(QUOTA_RESET_WAIT));
                    continue;
                }

                if !response.status().is_success() {
                    println!("API 请求失败: {} - {}", response.status(), response.text().await.unwrap_or_default());
                    if attempt < MAX_RETRIES - 1 {
                        let wait_time = 5 * (attempt + 1) as u64;
                        println!("等待 {} 秒后重试...", wait_time);
                        sleep(Duration::from_secs(wait_time));
                        continue;
                    }
                    return None;
                }

                if let Ok(json_response) = response.json::<serde_json::Value>().await {
                    if let Some(text) = json_response
                        .get("candidates")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("content"))
                        .and_then(|c| c.get("parts"))
                        .and_then(|p| p.get(0))
                        .and_then(|p| p.get("text"))
                        .and_then(|t| t.as_str()) {

                        // 解析返回的文本
                        let mut title = String::new();
                        let mut tags = Vec::new();

                        for line in text.lines() {
                            if line.starts_with("标题：") {
                                title = line.trim_start_matches("标题：").trim().to_string();
                            } else if line.starts_with("标签：") {
                                let tags_str = line.trim_start_matches("标签：").trim();
                                let delim_re = Regex::new(r"[,，、]").unwrap();
                                tags = delim_re
                                    .split(tags_str)
                                    .map(|s| s.trim().to_string())
                                    .filter(|t| {
                                        let len = t.chars().count();
                                        len >= 2 && len <= 15 &&
                                            !TAG_BLACKLIST.contains(&t.as_str()) &&
                                            !t.chars().any(|c| ['!', '?', '。', '；'].contains(&c))
                                    })
                                    .collect();
                            }
                        }

                        if !title.is_empty() && !tags.is_empty() {
                            return Some(AiResponse {
                                title,
                                tags: tags.into_iter().take(5).collect(),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                println!("API 调用错误: {}", e);
                if attempt < MAX_RETRIES - 1 {
                    let wait_time = 5 * (attempt + 1) as u64;
                    println!("等待 {} 秒后重试...", wait_time);
                    sleep(Duration::from_secs(wait_time));
                }
            }
        }

        sleep(Duration::from_secs_f64(REQUEST_INTERVAL));
    }
    None
}

async fn process_bookmarks(bookmarks: &mut Vec<Bookmark>, api_key: &str) -> Stats {
    let mut stats = Stats {
        total: 0,
        updated: 0,
        failed: 0,
    };
    let batch_size = 10;
    let batch_interval = Duration::from_secs(5);

    let total_bookmarks = bookmarks.len();
    for idx in 0..total_bookmarks {
        stats.total += 1;

        if idx > 0 && idx % batch_size == 0 {
            println!("批次处理完成，暂停 {} 秒...", batch_interval.as_secs());
            sleep(batch_interval);

            if let Err(e) = save_progress(&bookmarks, "bookmarks_progress.html") {
                println!("保存进度失败：{}", e);
            }
        }

        if !validate_url(&bookmarks[idx].url) {
            println!("无效URL [{}]：{}", idx, bookmarks[idx].url);
            stats.failed += 1;
            continue;
        }

        match get_tags_from_gemini(&bookmarks[idx].url, api_key).await {
            Some(ai_response) => {
                bookmarks[idx].title = ai_response.title;
                bookmarks[idx].tags = Some(ai_response.tags.clone());
                stats.updated += 1;
                println!("成功处理 [{}]：", idx);
                println!("  标题：{}", bookmarks[idx].title);
                println!("  标签：{:?}", ai_response.tags);
            }
            None => {
                stats.failed += 1;
                println!("失败 [{}]：未获取到有效内容", idx);
            }
        }
    }

    stats
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let api_key = match env::var("GOOGLE_API_KEY") {
        Ok(key) => {
            if key.is_empty() || key == "your_api_key_here" {
                eprintln!("错误：请在.env文件中设置有效的GOOGLE_API_KEY");
                std::process::exit(1);
            }
            key
        },
        Err(_) => {
            eprintln!("错误：未找到GOOGLE_API_KEY环境变量");
            std::process::exit(1);
        }
    };

    let input_file = "bookmarks.html";
    let html_content = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("读取书签文件失败：{}", err);
            std::process::exit(1);
        }
    };

    let mut bookmarks = parse_bookmarks_html(&html_content);
    println!("成功解析 {} 个书签", bookmarks.len());

    let start_time = Instant::now();
    let stats = process_bookmarks(&mut bookmarks, &api_key).await;

    let output_file = "bookmarks_with_tags.html";
    let output_html = generate_bookmarks_html(&bookmarks);
    if let Err(e) = fs::write(output_file, output_html) {
        eprintln!("保存书签文件失败：{}", e);
        std::process::exit(1);
    }

    println!("\n处理完成 | 耗时：{:.1?}", start_time.elapsed());
    println!("总计处理：{}", stats.total);
    println!("成功更新：{}", stats.updated);
    println!("失败条目：{}", stats.failed);
    println!("结果已保存至：{}", output_file);
}
