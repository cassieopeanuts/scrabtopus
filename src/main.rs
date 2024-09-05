use reqwest::blocking::get;
use scraper::{Html, Selector, ElementRef};
use std::error::Error;
use csv::Writer;
use std::fs::File;
use std::collections::HashSet;

fn scrape_page(url: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let body = get(url)?.text()?;
    let document = Html::parse_document(&body);

    // Selectors
    let title_selector = Selector::parse("h1").unwrap(); // Main page title
    let header_selector = Selector::parse("h4, h3").unwrap(); // Section headers like "Action"
    let paragraph_selector = Selector::parse("p").unwrap(); // Paragraph content
    let container_selector = Selector::parse("div, section, article").unwrap(); // Meaningful containers
    let content_selector = Selector::parse("p, div, span").unwrap(); // General content holders inside containers
    let exclusion_selector = Selector::parse("nav, button, footer, a").unwrap(); // Exclude irrelevant elements

    let mut results = Vec::new();
    let mut seen_content = HashSet::new(); // Track and avoid duplicating content
    let mut current_title = String::new();

    // Extract the main page title
    if let Some(main_title) = document.select(&title_selector).next() {
        current_title = main_title.text().collect::<Vec<_>>().join(" ").trim().to_string();
        results.push((current_title.clone(), String::new())); // Add title as the first entry
    }

    // Scrape paragraphs associated with headers (h4, h3) and collect content
    for header in document.select(&header_selector) {
        let section_title = header.text().collect::<Vec<_>>().join(" ").trim().to_string();
        let mut section_content = String::new();

        // Find sibling paragraphs following each header
        let mut next_sibling = header.next_sibling();

        while let Some(sibling) = next_sibling {
            if let Some(el) = ElementRef::wrap(sibling) {
                if paragraph_selector.matches(&el) {
                    let p_content = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    section_content.push_str(&p_content);
                    section_content.push(' '); // Add space between paragraphs
                } else if header_selector.matches(&el) {
                    break; // Stop when encountering another header
                }
            }
            next_sibling = sibling.next_sibling();
        }

        if !section_title.is_empty() && !section_content.is_empty() {
            results.push((section_title, section_content));
        }
    }

    // Scrape general containers like divs, sections, and articles
    for container in document.select(&container_selector) {
        // Ignore excluded elements like nav, buttons, etc.
        if exclusion_selector.matches(&container) {
            continue;
        }

        let mut container_content = String::new();
        let mut next_sibling = container.first_child(); // Start with the first child of the container

        while let Some(sibling) = next_sibling {
            if let Some(el) = ElementRef::wrap(sibling) {
                if content_selector.matches(&el) {
                    let p_content = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    // Avoid adding duplicate or empty content
                    if !p_content.is_empty() && seen_content.insert(p_content.clone()) {
                        container_content.push_str(&p_content);
                        container_content.push(' '); // Add space between paragraphs
                    }
                }
            }
            next_sibling = sibling.next_sibling();
        }

        if !container_content.is_empty() {
            results.push((current_title.clone(), container_content));
        }
    }

    Ok(results)
}

fn write_to_csv(data: Vec<(String, String)>, output_path: &str) -> Result<(), Box<dyn Error>> {
    let file = File::create(output_path)?;
    let mut wtr = Writer::from_writer(file);

    // Write the header row
    wtr.write_record(&["Title", "Content"])?;

    // Write each title-content pair to CSV
    for (title, content) in data {
        wtr.write_record(&[title, content])?;
    }

    wtr.flush()?;
    Ok(())
}

fn main() {
    let url = "https://www.holochain.org/roadmap/";  // Replace with the actual URL

    match scrape_page(url) {
        Ok(data) => {
            println!("Scraped data, writing to CSV...");
            match write_to_csv(data, "scraped_data.csv") {
                Ok(_) => println!("Data successfully written to scraped_data.csv"),
                Err(e) => println!("An error occurred while writing to CSV: {}", e),
            }
        }
        Err(e) => println!("An error occurred: {}", e),
    }
}
