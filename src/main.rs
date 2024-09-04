use reqwest::blocking::get;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::error::Error;
use csv::Writer;
use std::fs::File;

fn normalize_text(text: &str) -> String {
    text.to_lowercase().trim().to_string()  // Lowercase and trim whitespace
}

fn scrape_page(url: &str) -> Result<Vec<String>, Box<dyn Error>> {
    // Send HTTP GET request to the given URL
    let body = get(url)?.text()?;
    
    // Parse the HTML body
    let document = Html::parse_document(&body);

    // Selectors to extract data (modify these to fit your use case)
    let div_selector = Selector::parse("div, p, section, article").unwrap();  // Target main content elements
    let span_selector = Selector::parse("span").unwrap(); // Select all spans
    let button_selector = Selector::parse("button, .btn, .swiper-button-next, .swiper-button-prev, .swiper-pagination, form, input, .footer_menu_flex, .footer_menu, footer_logo_wrap, .join_preview, .stat_badge, .stat_section, .browser-warning-container, .cookie_modal, .close_modal").unwrap();  // Select buttons and UI elements to skip

    let mut data = Vec::new();
    let mut seen_content = HashSet::new();  // Use HashSet to track seen content

    // Extract text from divs, paragraphs, sections, and articles
    for element in document.select(&div_selector) {
        // Skip elements that are interactive buttons, forms, or pagination
        if element.select(&button_selector).count() > 0 {
            continue;
        }

        // Collect meaningful text from these elements
        let content = element.text().collect::<Vec<_>>().join(" ").trim().to_string();

        // Filter out buttons, cookie texts, and navigation-related keywords
        if !content.is_empty()

            && !content.contains("sign up")
            && !content.contains("privacy policy")

        {
            let normalized_content = normalize_text(&content);
            // Only add if this content hasn't been seen before
            if seen_content.insert(normalized_content.clone()) {
                data.push(content);  // Preserve original content order
            }
        }
    }

    // Optionally collect text from span elements as well, if needed
    for element in document.select(&span_selector) {
        let content = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
        if !content.is_empty() && !content.contains("cookie") && !content.contains("sign up") {
            let normalized_content = normalize_text(&content);
            if seen_content.insert(normalized_content.clone()) {
                data.push(content);
            }
        }
    }

    Ok(data)  // No sorting, content is in original order
}

fn write_to_csv(data: Vec<String>, output_path: &str) -> Result<(), Box<dyn Error>> {
    // Create a CSV writer to write to a file
    let file = File::create(output_path)?;
    let mut wtr = Writer::from_writer(file);

    // Write the header row (e.g., Title, Content)
    wtr.write_record(&["Title", "Content"])?;

    // Write each piece of content as a row in the CSV
    for (index, content) in data.iter().enumerate() {
        wtr.write_record(&[format!("Section {}", index + 1), content.to_string()])?;
    }

    // Flush the writer to ensure all data is written
    wtr.flush()?;

    Ok(())
}

fn main() {
    let url = "https://developer.holochain.org/resources/glossary/";  // Replace with the URL you want to scrape
    
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
