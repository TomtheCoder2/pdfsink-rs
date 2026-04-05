use pdfsink_rs::{PdfDocument, TableSettings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = PdfDocument::open("tests/fixtures/simple_text.pdf")?;
    let page = pdf.page(1)?;

    println!("text:\n{}", page.extract_text());
    println!("word count: {}", page.extract_words().len());

    let tables = page.extract_tables(TableSettings::default())?;
    println!("table count: {}", tables.len());

    Ok(())
}
