use chrono::Utc;
use htmlescape::encode_minimal;
use scraper::{ElementRef, Html, Selector};
use std::env;
use std::fs::File;
use std::io::Write;

#[derive(Debug)]
struct Notice {
    index: i32,
    title: String,
    author: String,
    category: String,
    link: String,
    published_at: String,
}

fn fetch_html(base_url: &str, limit: u8, offset: u8) -> String {
    let url = format!(
        "{}?mode=list&articleLimit={}&article.offset={}",
        base_url, limit, offset
    );

    let res = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .unwrap();

    assert!(res.status().is_success());

    res.text().unwrap()
}

fn parse_text(row: &ElementRef, selector: &Selector) -> String {
    row.select(&selector)
        .flat_map(|datum| datum.text().collect::<Vec<_>>())
        .map(|datum| datum.trim().replace("\n", "").replace("\t", ""))
        .filter(|datum| !datum.is_empty())
        .collect::<Vec<_>>()
        .first()
        .unwrap_or(&String::from(""))
        .clone()
}

fn parse_attr(row: &ElementRef, selector: &Selector) -> String {
    row.select(&selector)
        .flat_map(|datum| datum.value().attr("href"))
        .collect::<Vec<_>>()
        .first()
        .unwrap_or(&"")
        .to_string()
}

fn parse_html(html: &str, base_url: &str) -> Vec<Notice> {
    let fragment = Html::parse_document(html);
    let row_selector = Selector::parse("table.board-table > tbody > tr").unwrap();

    fragment
        .select(&row_selector)
        .map(|row| -> Notice {
            let index_selector = Selector::parse("td.b-num-box").unwrap();
            let category_selector = Selector::parse("td.b-num-box + td").unwrap();
            let title_selector = Selector::parse("td.b-td-left > div.b-title-box").unwrap();
            let link_selector = Selector::parse("td.b-td-left > div.b-title-box > a").unwrap();
            let author_selector = Selector::parse("td.b-no-right + td").unwrap();
            let published_at_selector = Selector::parse("td.b-no-right + td + td").unwrap();

            Notice {
                index: parse_text(&row, &index_selector)
                    .parse::<i32>()
                    .unwrap_or(-1),
                category: encode_minimal(&parse_text(&row, &category_selector)),
                title: encode_minimal(&parse_text(&row, &title_selector)),
                author: encode_minimal(&parse_text(&row, &author_selector)),
                link: encode_minimal(&format!("{}{}", base_url, parse_attr(&row, &link_selector))),
                published_at: encode_minimal(&parse_text(&row, &published_at_selector)),
            }
        })
        .collect::<Vec<_>>()
}

fn compose_xml(notices: &[Notice]) -> String {
    let header = format!(
        "<rss version=\"2.0\">\n \
                  <channel>\n \
                  <title>Ajou University Notices</title>\n \
                  <link>https://ajou.ac.kr/kr/ajou/notice.do</link>\n \
                  <description>Recently published notices</description>\n \
                  <language>ko-kr</language>\n \
                  <lastBuildDate>{}</lastBuildDate>",
        Utc::now().to_rfc2822()
    );

    let footer = "</channel>\n \
                  </rss>";

    let items = notices
        .iter()
        .map(|notice| -> String {
            let description = format!(
                "[{}] - {} ({})",
                notice.category, notice.author, notice.published_at
            );
            format!(
                "<item>\n \
                <title>{}</title>\n \
                <link>{}</link>\n \
                <description>{}</description>\n \
                </item>",
                notice.title, notice.link, description
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    format!("{}\n{}\n{}", header, items, footer)
}

fn compose_md(notices: &[Notice]) -> String {
    let header = "# 최근 공지사항";

    let items = notices
        .iter()
        .map(|notice| -> String {
            let description = format!(
                "[{}] - {} (~{})",
                notice.category, notice.author, notice.published_at
            );
            format!(
                r"* **[{}]({})**\n  {}",
                notice.title, notice.link, description
            )
        })
        .collect::<Vec<String>>()
        .join(r"\n\n");

    format!(r"{}\n\n{}", header, items)
}

fn compose_commit_message(notices: &[Notice], last_index: i32) -> String {
    let header = format!("dist: {}", last_index);

    let items = notices
        .iter()
        .map(|notice| format!("* {}", notice.title))
        .collect::<Vec<String>>()
        .join("\n");

    format!("{}\n\n{}", header, items)
}

fn write_last_index(last_index: i32) {
    let current_exe = env::current_exe().unwrap();
    let current_dir = current_exe.parent().unwrap();
    let path = format!("{}/last_index", current_dir.display());
    let mut file = File::create(&path).unwrap();
    file.write_all(last_index.to_string().as_bytes()).unwrap();
}

fn main() {
    const BASE_URL: &str = "http://ajou.ac.kr/kr/ajou/notice.do";
    const LIMIT: u8 = 30;
    const OFFSET: u8 = 0;

    let args = env::args().collect::<Vec<String>>();
    let last_index = args[1].parse::<i32>().unwrap();
    let mode = args[2]
        .parse::<String>()
        .unwrap_or_else(|_| "xml".to_string());

    let html = fetch_html(BASE_URL, LIMIT, OFFSET);
    let notices = parse_html(&html, BASE_URL);
    let latest_index = notices
        .iter()
        .filter(|notice| notice.index != -1)
        .collect::<Vec<_>>()
        .first()
        .unwrap()
        .index;

    if last_index != latest_index {
        match mode.as_str() {
            "xml" => println!("{}", compose_xml(&notices)),
            "md" => println!("{}", compose_md(&notices)),
            "cm" => println!("{}", compose_commit_message(&notices, last_index)),
            _ => eprintln!("unknown mode '{}'", mode),
        }

        write_last_index(latest_index);
    } else {
        eprintln!("new notices not found")
    }
}
