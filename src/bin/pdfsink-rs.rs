use image::ImageFormat;
use pdfsink_rs::{
    PdfDocument, Result, SearchOptions, TableSettings, TableStrategy, TextOptions,
};
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    let Some(path) = args.next() else {
        print_usage();
        return Ok(());
    };

    let pdf = PdfDocument::open(&path)?;

    match command.as_str() {
        "info" => {
            let payload = serde_json::json!({
                "path": path,
                "page_count": pdf.len(),
                "pages": pdf.pages().iter().map(|page| serde_json::json!({
                    "page_number": page.page_number,
                    "rotation": page.rotation,
                    "width": page.width,
                    "height": page.height,
                    "bbox": page.bbox,
                    "object_counts": page.object_counts(),
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        "text" => {
            let page = resolve_page(&pdf, args.next())?;
            println!("{}", page.extract_text());
        }
        "words" => {
            let page = resolve_page(&pdf, args.next())?;
            let words = page.extract_words();
            println!("{}", serde_json::to_string_pretty(&words)?);
        }
        "search" => {
            let page = resolve_page(&pdf, args.next())?;
            let pattern = args.next().unwrap_or_default();
            let matches = page.search_with_options(&pattern, &SearchOptions::default(), &TextOptions::default())?;
            println!("{}", serde_json::to_string_pretty(&matches)?);
        }
        "objects" => {
            let page = resolve_page(&pdf, args.next())?;
            let payload = page.to_dict(None);
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        "json" => {
            let page = resolve_page(&pdf, args.next())?;
            let rendered = page.to_json::<Vec<u8>>(None, None, None, None, None, Some(2))?
                .unwrap_or_default();
            println!("{}", rendered);
        }
        "csv" => {
            let page = resolve_page(&pdf, args.next())?;
            let rendered = page.to_csv::<Vec<u8>>(None, None, None, None, None)?
                .unwrap_or_default();
            println!("{}", rendered);
        }
        "links" => {
            let page = resolve_page(&pdf, args.next())?;
            println!("{}", serde_json::to_string_pretty(&page.hyperlinks)?);
        }
        "table" => {
            let page = resolve_page(&pdf, args.next())?;
            let strategy = args.next().unwrap_or_else(|| "lines".to_string());
            let strat: TableStrategy = strategy.parse()?;
            let mut settings = TableSettings::default();
            settings.vertical_strategy = strat;
            settings.horizontal_strategy = strat;
            let table = page.extract_table(settings)?;
            println!("{}", serde_json::to_string_pretty(&table)?);
        }
        "svg" => {
            let page = resolve_page(&pdf, args.next())?;
            let output = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("page.svg"));
            fs::write(&output, page.to_debug_svg())?;
            eprintln!("wrote {}", output.display());
        }
        "render" => {
            let page = resolve_page(&pdf, args.next())?;
            let output = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("page.png"));
            let image = page.to_image(Some(150.0), None, None, false, false)?;
            image.save(&output, Some(ImageFormat::Png), false, 256, 8)?;
            eprintln!("wrote {}", output.display());
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn resolve_page<'a>(pdf: &'a PdfDocument, page_arg: Option<String>) -> Result<&'a pdfsink_rs::Page> {
    let page_number = page_arg
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    pdf.page(page_number)
}

fn print_usage() {
    eprintln!(
        "\
pdfsink-rs

Usage:
  pdfsink-rs info <file.pdf>
  pdfsink-rs text <file.pdf> [page]
  pdfsink-rs words <file.pdf> [page]
  pdfsink-rs search <file.pdf> [page] [pattern]
  pdfsink-rs objects <file.pdf> [page]
  pdfsink-rs json <file.pdf> [page]
  pdfsink-rs csv <file.pdf> [page]
  pdfsink-rs links <file.pdf> [page]
  pdfsink-rs table <file.pdf> [page] [lines|lines_strict|text|explicit]
  pdfsink-rs svg <file.pdf> [page] [output.svg]
  pdfsink-rs render <file.pdf> [page] [output.png]
"
    );
}
