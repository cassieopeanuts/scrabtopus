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

    // Refined Selector - Targeting specific sections like `section` or `.main-area` 
    let section_selector = Selector::parse("section, .main-area, .content").unwrap();  
    let button_selector = Selector::parse("button, .btn, form, input, footer").unwrap();  // Skip interactive UI elements

    let mut data = Vec::new();
    let mut seen_content = HashSet::new();  // Use HashSet to track seen content

    // Extract text from sections or main content areas
    for element in document.select(&section_selector) {
        // Skip elements that are buttons, forms, etc.
        if element.select(&button_selector).count() > 0 {
            continue;
        }

        // Collect meaningful text
        let content = element.text().collect::<Vec<_>>().join(" ").trim().to_string();

        // Filter out non-content text
        if !content.is_empty()
            && !content.contains("sign up")
            && !content.contains("privacy policy")
        {
            let normalized_content = normalize_text(&content);
            // Avoid duplications by checking if content is already seen
            if seen_content.insert(normalized_content.clone()) {
                data.push(content);  // Store unique content
            }
        }
    }

    Ok(data)  // Return the collected data
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
