use reqwest::blocking::Client;
use scraper::{Html, Selector, ElementRef};
use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::collections::{HashSet, VecDeque};
use url::Url;

/// Structs for JSON serialization
#[derive(Serialize, Debug)]
struct ScrapedData {
    pages: Vec<Page>,
}

#[derive(Serialize, Debug)]
struct Page {
    url: String,
    title: String,
    sections: Vec<Section>,
}

#[derive(Serialize, Debug)]
struct Section {
    header: String,
    content: Vec<Content>,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum Content {
    Paragraph(String),
    List { lists: Vec<String> },
}

/// Scrapes meaningful text from a single page.
/// It targets the <main> tag and excludes buttons and policy-related sections.
fn scrape_page(document: &Html, url: &Url) -> Result<Page, Box<dyn Error>> {
    // Selectors
    let main_selector = Selector::parse("main").unwrap(); // Main content
    let title_selector = Selector::parse("h1, h2, h3, h4").unwrap(); // Titles and headers
    let paragraph_selector = Selector::parse("p").unwrap(); // Paragraphs
    let list_selector = Selector::parse("ul, ol").unwrap(); // Lists
    let list_item_selector = Selector::parse("li").unwrap(); // List items
    let exclusion_selector = Selector::parse("button, nav, footer, a, script, style, svg, img").unwrap(); // Exclude these elements

    // Initialize page struct
    let mut page = Page {
        url: url.as_str().to_string(),
        title: String::new(),
        sections: Vec::new(),
    };

    // Extract the main content
    if let Some(main_content) = document.select(&main_selector).next() {
        // Extract the main title
        if let Some(title_elem) = main_content.select(&title_selector).next() {
            let title_text = title_elem.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if !title_text.is_empty() {
                page.title = title_text;
            }
        }

        // Iterate over headers to define sections
        for header in main_content.select(&title_selector) {
            let header_text = header.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if header_text.is_empty() {
                continue;
            }

            let mut section = Section {
                header: header_text.clone(),
                content: Vec::new(),
            };

            // Traverse siblings to collect content until the next header
            let mut sibling = header.next_sibling();
            while let Some(sib) = sibling {
                if let Some(element) = ElementRef::wrap(sib) {
                    // If another header is encountered, end this section
                    if title_selector.matches(&element) {
                        break;
                    }

                    // Skip excluded elements
                    if exclusion_selector.matches(&element) {
                        sibling = sib.next_sibling();
                        continue;
                    }

                    // Extract paragraphs
                    if paragraph_selector.matches(&element) {
                        let para_text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        if !para_text.is_empty() {
                            section.content.push(Content::Paragraph(para_text));
                        }
                    }

                    // Extract lists
                    if list_selector.matches(&element) {
                        let mut items = Vec::new();
                        for li in element.select(&list_item_selector) {
                            let li_text = li.text().collect::<Vec<_>>().join(" ").trim().to_string();
                            if !li_text.is_empty() {
                                items.push(li_text);
                            }
                        }
                        if !items.is_empty() {
                            section.content.push(Content::List { lists: items });
                        }
                    }
                }
                sibling = sib.next_sibling();
            }

            if !section.content.is_empty() {
                page.sections.push(section);
            }
        }
    }

    Ok(page)
}

/// Extracts all internal links from the <main> section of a page.
/// Excludes links to policies and external sites.
fn extract_internal_links(document: &Html, base_url: &Url) -> Result<Vec<Url>, Box<dyn Error>> {
    let main_selector = Selector::parse("main").unwrap();
    let link_selector = Selector::parse("a[href]").unwrap();

    let mut links = Vec::new();

    if let Some(main_content) = document.select(&main_selector).next() {
        for element in main_content.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                // Resolve relative URLs
                if let Ok(resolved_url) = base_url.join(href) {
                    // Ensure the link is within the same domain
                    if let Some(base_domain) = base_url.domain() {
                        if resolved_url.domain() == Some(base_domain) {
                            // Exclude policy-related links based on common substrings
                            let path = resolved_url.path().to_lowercase();
                            if !path.contains("policy")
                                && !path.contains("terms")
                                && !path.contains("cookie")
                                && !path.contains("privacy")
                                && !path.contains("license")
                            {
                                links.push(resolved_url);
                            }
                        }
                    }
                }
            }
        }
    }

    // Remove duplicates
    links.sort();
    links.dedup();

    Ok(links)
}

/// Writes the scraped data to a JSON file.
fn write_to_json(data: ScrapedData, output_path: &str) -> Result<(), Box<dyn Error>> {
    let file = File::create(output_path)?;
    serde_json::to_writer_pretty(file, &data)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the HTTP client with a custom User-Agent
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; RustScraper/1.0)")
        .build()?;

    let start_url = "https://blog.holochain.org/"; // Starting URL
    let base_url = Url::parse(start_url)?;

    // Initialize queue and visited set
    let mut queue: VecDeque<Url> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut scraped_data = ScrapedData { pages: Vec::new() };

    // Start with the initial URL
    queue.push_back(base_url.clone());

    // Define the maximum number of pages to scrape to prevent infinite loops (optional)
    let max_pages = 200;
    let mut pages_scraped = 0;

    while let Some(current_url) = queue.pop_front() {
        if pages_scraped >= max_pages {
            println!("Reached the maximum limit of {} pages.", max_pages);
            break;
        }

        let url_str = current_url.as_str().to_string();
        if visited.contains(&url_str) {
            continue; // Skip if already visited
        }

        println!("Scraping: {}", current_url);
        match client.get(current_url.clone()).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text() {
                        Ok(body) => {
                            let document = Html::parse_document(&body);

                            // Scrape the page
                            match scrape_page(&document, &current_url) {
                                Ok(page) => {
                                    scraped_data.pages.push(page);
                                    pages_scraped += 1;
                                },
                                Err(e) => eprintln!("Error scraping {}: {}", current_url, e),
                            }

                            // Extract and enqueue internal links
                            match extract_internal_links(&document, &current_url) {
                                Ok(links) => {
                                    for link in links {
                                        let link_str = link.as_str().to_string();
                                        if !visited.contains(&link_str) && !queue.contains(&link) {
                                            queue.push_back(link);
                                        }
                                    }
                                },
                                Err(e) => eprintln!("Error extracting links from {}: {}", current_url, e),
                            }
                        },
                        Err(e) => eprintln!("Error reading body from {}: {}", current_url, e),
                    }
                } else {
                    eprintln!("Non-success status code {} for URL: {}", response.status(), current_url);
                }
            },
            Err(e) => eprintln!("Request error for {}: {}", current_url, e),
        }

        // Mark as visited
        visited.insert(url_str);

        // Optional: Rate limiting to prevent overwhelming the server
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Write all collected data to JSON
    println!("Scraped {} pages, writing data to JSON...", pages_scraped);
    match write_to_json(scraped_data, "scraped_data.json") {
        Ok(_) => println!("Data successfully written to scraped_data.json"),
        Err(e) => eprintln!("An error occurred while writing to JSON: {}", e),
    }

    Ok(())
}
