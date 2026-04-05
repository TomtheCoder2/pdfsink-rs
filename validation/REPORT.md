# pdfsink-rs reference validation package

This package was assembled as a native pure-Rust reimplementation attempt of `pdfplumber`.

Included reference material:

- 7 generated fixture PDFs in `tests/fixtures`
- `tests/goldens/goldens.json`, produced by running `pdfplumber` over those fixtures
- `tests/golden.rs`, a Rust integration test suite that compares the native implementation to those goldens

Reference checks covered by the goldens:

- page count and page geometry
- object counts for chars / lines / rects / curves / images / annotations / hyperlinks
- exact extracted text on all fixtures
- word extraction order on all fixtures
- search output for `simple_text.pdf`
- crop and outside-bbox behavior on `crop_regions.pdf`
- line-based table extraction on `table_lines.pdf`
- text-strategy table extraction on `table_text_only.pdf`
- image / path / hyperlink object summaries on `objects_showcase.pdf`
- duplicate-character removal on `rotated_and_duplicates.pdf`
- multi-page extraction on `multipage.pdf`

Constraint in this environment:

- No Rust toolchain was available, so the crate could not be compiled or executed here.
- The packaged source, tests, fixtures, and goldens are present so the crate can be compiled and verified in a normal Rust environment.
