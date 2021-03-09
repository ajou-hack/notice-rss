#[derive(Debug)]
struct Notice {
    index: String,
    title: String,
    author: String,
    category: String,
    link: String,
    published_at: String,
}

fn fetch_html(base_url: &str, limit: u8, offset: u8) -> String {
    let url = format!("{}?mode=list&articleLimit={}&article.offset={}", base_url, limit, offset);
    let res = reqwest::blocking::Client::new().get(&url).header("User-Agent", "Mozilla/5.0").send().unwrap();

    assert!(res.status().is_success());

    res.text().unwrap()
}

fn parse_text(row: &scraper::ElementRef, selector: &scraper::Selector) -> String {
    row.select(&selector)
        .flat_map(|datum| datum.text().collect::<Vec<_>>())
        .map(|datum| datum.trim().replace("\n", "").replace("\t", ""))
        .filter(|datum| datum.len() > 0)
        .collect::<Vec<_>>()
        .first()
        .unwrap_or(&String::from(""))
        .clone()
}

fn parse_attr(row: &scraper::ElementRef, selector: &scraper::Selector) -> String {
    row.select(&selector)
        .flat_map(|datum| datum.value().attr("href"))
        .collect::<Vec<_>>()
        .first()
        .unwrap_or(&"")
        .to_string()
}


fn parse_html(html: &str, base_url: &str) -> Vec<Notice> {
    let fragment = scraper::Html::parse_document(html);
    let row_selector = scraper::Selector::parse("table.board-table > tbody > tr").unwrap();

    fragment.select(&row_selector).map(|row| -> Notice {
        let index_selector = scraper::Selector::parse("td.b-num-box").unwrap();
        let category_selector = scraper::Selector::parse("td.b-num-box + td").unwrap();
        let title_selector = scraper::Selector::parse("td.b-td-left > div.b-title-box").unwrap();
        let link_selector = scraper::Selector::parse("td.b-td-left > div.b-title-box > a").unwrap();
        let author_selector = scraper::Selector::parse("td.b-no-right + td").unwrap();
        let published_at_selector = scraper::Selector::parse("td.b-no-right + td + td").unwrap();

        Notice {
            index: parse_text(&row, &index_selector),
            category: parse_text(&row, &category_selector),
            title: parse_text(&row, &title_selector),
            author: parse_text(&row, &author_selector),
            link: format!("{}{}", base_url, parse_attr(&row, &link_selector)), 
            published_at: parse_text(&row, &published_at_selector),
        }
    }).collect::<Vec<_>>()
}

fn main() {
    const BASE_URL: &str = "https://ajou.ac.kr/kr/ajou/notice.do";

    let html = fetch_html(BASE_URL, 5, 0);
    let notices = parse_html(&html, BASE_URL);

    println!("{:?}", notices);
}
