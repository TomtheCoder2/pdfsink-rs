use crate::clustering::cluster_items;
use crate::geometry::objects_to_bbox;
use crate::types::{BBox, Char, Direction, SearchMatch, TextLine, Word};

#[derive(Debug, Clone)]
pub struct TextOptions {
    pub x_tolerance: f64,
    pub y_tolerance: f64,
    pub x_tolerance_ratio: Option<f64>,
    pub y_tolerance_ratio: Option<f64>,
    pub layout: bool,
    pub layout_width: Option<f64>,
    pub layout_height: Option<f64>,
    pub layout_width_chars: Option<usize>,
    pub layout_height_chars: Option<usize>,
    pub layout_bbox: Option<BBox>,
    pub x_density: f64,
    pub y_density: f64,
    pub x_shift: f64,
    pub y_shift: f64,
    pub line_dir: Direction,
    pub char_dir: Direction,
    pub line_dir_rotated: Option<Direction>,
    pub char_dir_rotated: Option<Direction>,
    pub line_dir_render: Option<Direction>,
    pub char_dir_render: Option<Direction>,
    pub keep_blank_chars: bool,
    pub use_text_flow: bool,
    pub split_at_punctuation: Option<String>,
    pub expand_ligatures: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            x_tolerance: 3.0,
            y_tolerance: 3.0,
            x_tolerance_ratio: None,
            y_tolerance_ratio: None,
            layout: false,
            layout_width: None,
            layout_height: None,
            layout_width_chars: None,
            layout_height_chars: None,
            layout_bbox: None,
            x_density: 7.25,
            y_density: 13.0,
            x_shift: 0.0,
            y_shift: 0.0,
            line_dir: Direction::Ttb,
            char_dir: Direction::Ltr,
            line_dir_rotated: None,
            char_dir_rotated: None,
            line_dir_render: None,
            char_dir_render: None,
            keep_blank_chars: false,
            use_text_flow: false,
            split_at_punctuation: None,
            expand_ligatures: true,
        }
    }
}

impl TextOptions {
    pub fn with_tolerances(mut self, x_tolerance: f64, y_tolerance: f64) -> Self {
        self.x_tolerance = x_tolerance;
        self.y_tolerance = y_tolerance;
        self
    }

    pub fn with_tolerance_ratios(mut self, x_tolerance_ratio: Option<f64>, y_tolerance_ratio: Option<f64>) -> Self {
        self.x_tolerance_ratio = x_tolerance_ratio;
        self.y_tolerance_ratio = y_tolerance_ratio;
        self
    }

    pub fn with_layout(mut self, layout: bool) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_layout_bbox(mut self, bbox: BBox) -> Self {
        self.layout_bbox = Some(bbox);
        self.layout_width = Some(bbox.width());
        self.layout_height = Some(bbox.height());
        self
    }

    pub fn with_directions(mut self, line_dir: Direction, char_dir: Direction) -> Self {
        self.line_dir = line_dir;
        self.char_dir = char_dir;
        self
    }

    pub fn with_split_at_punctuation(mut self, punctuation: impl Into<String>) -> Self {
        self.split_at_punctuation = Some(punctuation.into());
        self
    }

    pub fn with_keep_blank_chars(mut self, keep_blank_chars: bool) -> Self {
        self.keep_blank_chars = keep_blank_chars;
        self
    }

    pub fn with_text_flow(mut self, use_text_flow: bool) -> Self {
        self.use_text_flow = use_text_flow;
        self
    }

    pub fn with_expand_ligatures(mut self, expand_ligatures: bool) -> Self {
        self.expand_ligatures = expand_ligatures;
        self
    }

    pub fn resolved_line_dir_rotated(&self) -> Direction {
        self.line_dir_rotated.unwrap_or(self.char_dir)
    }

    pub fn resolved_char_dir_rotated(&self) -> Direction {
        self.char_dir_rotated.unwrap_or(self.line_dir)
    }

    pub fn resolved_line_dir_render(&self) -> Direction {
        self.line_dir_render.unwrap_or(self.line_dir)
    }

    pub fn resolved_char_dir_render(&self) -> Direction {
        self.char_dir_render.unwrap_or(self.char_dir)
    }
}

#[derive(Debug, Clone)]
pub struct DedupeOptions {
    pub tolerance: f64,
    pub extra_attrs: Vec<String>,
}

impl Default for DedupeOptions {
    fn default() -> Self {
        Self {
            tolerance: 1.0,
            extra_attrs: vec!["fontname".to_string(), "size".to_string()],
        }
    }
}

impl DedupeOptions {
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    pub fn with_extra_attrs<I, S>(mut self, extra_attrs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_attrs = extra_attrs.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub regex: bool,
    pub case_sensitive: bool,
    pub main_group: usize,
    pub return_groups: bool,
    pub return_chars: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            regex: true,
            case_sensitive: true,
            main_group: 0,
            return_groups: true,
            return_chars: true,
        }
    }
}

impl SearchOptions {
    pub fn literal() -> Self {
        Self {
            regex: false,
            ..Self::default()
        }
    }

    pub fn case_insensitive(mut self) -> Self {
        self.case_sensitive = false;
        self
    }

    pub fn main_group(mut self, main_group: usize) -> Self {
        self.main_group = main_group;
        self
    }

    pub fn return_groups(mut self, return_groups: bool) -> Self {
        self.return_groups = return_groups;
        self
    }

    pub fn return_chars(mut self, return_chars: bool) -> Self {
        self.return_chars = return_chars;
        self
    }
}

#[derive(Debug, Clone)]
pub struct WordMap {
    pub tuples: Vec<(Word, Vec<Char>)>,
}

impl WordMap {
    pub fn to_textmap(&self, options: &TextOptions) -> TextMap {
        let mut tuples: Vec<(char, Option<Char>)> = Vec::new();
        if self.tuples.is_empty() {
            return TextMap {
                tuples,
                line_dir_render: options.resolved_line_dir_render(),
                char_dir_render: options.resolved_char_dir_render(),
            };
        }
        tuples.reserve(self.tuples.iter().map(|(word, _)| word.text.chars().count() + 1).sum());

        let mut width_chars = options.layout_width_chars.unwrap_or(0);
        if width_chars == 0 {
            if let Some(width) = options.layout_width {
                width_chars = (width / options.x_density).round() as usize;
            }
        }

        let mut height_chars = options.layout_height_chars.unwrap_or(0);
        if height_chars == 0 {
            if let Some(height) = options.layout_height {
                height_chars = (height / options.y_density).round() as usize;
            }
        }

        let layout_bbox = options.layout_bbox.unwrap_or_else(|| {
            let words: Vec<Word> = self.tuples.iter().map(|(word, _)| word.clone()).collect();
            objects_to_bbox(&words).unwrap_or_default()
        });

        let blank_line: Vec<(char, Option<Char>)> = if options.layout {
            vec![(' ', None); width_chars]
        } else {
            Vec::new()
        };

        let words_sorted = {
            let mut items = self.tuples.clone();
            items.sort_by(|a, b| {
                let va = line_cluster_value(&a.0, options.line_dir);
                let vb = line_cluster_value(&b.0, options.line_dir);
                va.total_cmp(&vb)
            });
            items
        };

        let line_tuples = cluster_items(
            &words_sorted,
            |pair| line_cluster_value(&pair.0, options.line_dir),
            options.y_tolerance,
        );

        let line_position_key = position_key_from_bbox(layout_bbox, options.line_dir);
        let char_position_origin = position_key_from_bbox(layout_bbox, options.char_dir);

        let mut num_newlines = 0isize;

        for (line_index, mut line) in line_tuples.into_iter().enumerate() {
            if !options.use_text_flow {
                line.sort_by(|a, b| {
                    let ka = sort_key(&a.0, options.char_dir);
                    let kb = sort_key(&b.0, options.char_dir);
                    ka.0.total_cmp(&kb.0).then_with(|| ka.1.total_cmp(&kb.1))
                });
            }

            let y_dist = if options.layout {
                let line_position = position_value(&line[0].0, options.line_dir);
                let raw = line_position - (line_position_key + options.y_shift);
                let adj = if matches!(options.line_dir, Direction::Btt | Direction::Rtl) {
                    -1.0
                } else {
                    1.0
                };
                raw * adj / options.y_density
            } else {
                0.0
            };

            let target_newlines = if line_index > 0 { 1 } else { 0 };
            let prepend = std::cmp::max(target_newlines, (y_dist.round() as isize) - num_newlines);

            for _ in 0..prepend.max(0) as usize {
                if tuples.is_empty() || tuples.last().map(|(c, _)| *c == '\n').unwrap_or(false) {
                    tuples.extend(blank_line.clone());
                }
                tuples.push(('\n', None));
            }
            num_newlines += prepend.max(0);

            let mut line_len: isize = 0;
            for (word, chars) in line {
                let x_dist = if options.layout {
                    let char_position = position_value(&word, options.char_dir);
                    let raw = char_position - (char_position_origin + options.x_shift);
                    let adj = if matches!(options.char_dir, Direction::Btt | Direction::Rtl) {
                        -1.0
                    } else {
                        1.0
                    };
                    raw * adj / options.x_density
                } else {
                    0.0
                };

                let prepend_spaces = std::cmp::max(std::cmp::min(1, line_len), (x_dist.round() as isize) - line_len);
                for _ in 0..prepend_spaces.max(0) as usize {
                    tuples.push((' ', None));
                }
                line_len += prepend_spaces.max(0);

                for ch in chars {
                    let expanded = expand_ligature_text(&ch.text, options.expand_ligatures);
                    for letter in expanded.chars() {
                        tuples.push((letter, Some(ch.clone())));
                        line_len += 1;
                    }
                }
            }

            if options.layout && width_chars > 0 && line_len < width_chars as isize {
                for _ in 0..(width_chars as isize - line_len) as usize {
                    tuples.push((' ', None));
                }
            }
        }

        if options.layout && height_chars > 0 {
            let append = height_chars as isize - (num_newlines + 1);
            for i in 0..append.max(0) as usize {
                if i > 0 {
                    tuples.extend(blank_line.clone());
                }
                tuples.push(('\n', None));
            }
            if tuples.last().map(|(c, _)| *c == '\n').unwrap_or(false) {
                tuples.pop();
            }
        }

        TextMap {
            tuples,
            line_dir_render: options.resolved_line_dir_render(),
            char_dir_render: options.resolved_char_dir_render(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextMap {
    pub tuples: Vec<(char, Option<Char>)>,
    pub line_dir_render: Direction,
    pub char_dir_render: Direction,
}

impl TextMap {
    pub fn len(&self) -> usize {
        self.tuples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tuples.is_empty()
    }

    pub fn base_string(&self) -> String {
        self.tuples.iter().map(|(c, _)| *c).collect()
    }

    pub fn as_string(&self) -> String {
        let base = self.base_string();
        if self.char_dir_render == Direction::Ltr && self.line_dir_render == Direction::Ttb {
            return base;
        }

        let mut lines: Vec<String> = base.lines().map(|line| line.to_string()).collect();

        if matches!(self.line_dir_render, Direction::Btt | Direction::Rtl) {
            lines.reverse();
        }

        if self.char_dir_render == Direction::Rtl {
            lines = lines.into_iter().map(|line| line.chars().rev().collect()).collect();
        }

        if matches!(self.line_dir_render, Direction::Rtl | Direction::Ltr) {
            let max_line_len = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
            let padded: Vec<Vec<char>> = lines
                .iter()
                .map(|line| {
                    let mut chars: Vec<char> = line.chars().collect();
                    while chars.len() < max_line_len {
                        if self.char_dir_render == Direction::Btt {
                            chars.insert(0, ' ');
                        } else {
                            chars.push(' ');
                        }
                    }
                    chars
                })
                .collect();

            let mut out = String::new();
            for idx in 0..max_line_len {
                for row in &padded {
                    out.push(row[idx]);
                }
                if idx + 1 != max_line_len {
                    out.push('\n');
                }
            }
            return out;
        }

        lines.join("\n")
    }

    pub fn extract_text_lines(&self, strip: bool, return_chars: bool) -> Vec<TextLine> {
        // Use the base string (1:1 char-to-tuple mapping) for offset tracking.
        let text: String = self.tuples.iter().map(|(c, _)| *c).collect();
        let mut out = Vec::new();
        let mut offset = 0usize;
        for raw_line in text.split('\n') {
            let line = if strip { raw_line.trim() } else { raw_line };
            let char_count = raw_line.chars().count();
            if line.is_empty() {
                offset += char_count + 1;
                continue;
            }

            let chars: Vec<Char> = self
                .slice_chars(offset, offset + char_count)
                .into_iter()
                .collect();

            if let Some(bbox) = objects_to_bbox(&chars) {
                out.push(TextLine {
                    text: line.to_string(),
                    x0: bbox.x0,
                    top: bbox.top,
                    x1: bbox.x1,
                    bottom: bbox.bottom,
                    chars: if return_chars { Some(chars) } else { None },
                });
            }
            offset += char_count + 1;
        }
        out
    }

    pub fn search(&self, pattern: &str, options: &SearchOptions) -> crate::Result<Vec<SearchMatch>> {
        let regex = if options.regex {
            regex::RegexBuilder::new(pattern)
                .case_insensitive(!options.case_sensitive)
                .build()?
        } else {
            regex::RegexBuilder::new(&regex::escape(pattern))
                .case_insensitive(!options.case_sensitive)
                .build()?
        };
        Ok(self.search_compiled(&regex, options))
    }

    pub fn search_compiled(&self, regex: &regex::Regex, options: &SearchOptions) -> Vec<SearchMatch> {
        // Use the base string (1:1 char-to-tuple mapping) so that byte/char
        // indices produced by the regex correspond directly to tuple positions.
        // as_string() may reorder lines and add padding, breaking the mapping.
        let haystack = self.base_string();
        let mut out = Vec::new();

        for captures in regex.captures_iter(&haystack) {
            let Some(main) = captures.get(options.main_group) else {
                continue;
            };
            if main.as_str().trim().is_empty() {
                continue;
            }

            let start = byte_to_char_index(&haystack, main.start());
            let end = byte_to_char_index(&haystack, main.end());

            let chars = self.slice_chars(start, end);
            if chars.is_empty() {
                continue;
            }
            let Some(bbox) = objects_to_bbox(&chars) else {
                continue;
            };

            let groups = if options.return_groups {
                let mut gs = Vec::with_capacity(captures.len().saturating_sub(1));
                for idx in 1..captures.len() {
                    gs.push(captures.get(idx).map(|m| m.as_str().to_string()));
                }
                Some(gs)
            } else {
                None
            };

            out.push(SearchMatch {
                text: main.as_str().to_string(),
                x0: bbox.x0,
                top: bbox.top,
                x1: bbox.x1,
                bottom: bbox.bottom,
                groups,
                chars: if options.return_chars { Some(chars) } else { None },
            });
        }

        out
    }

    fn slice_chars(&self, start: usize, end: usize) -> Vec<Char> {
        let start = start.min(self.tuples.len());
        let end = end.min(self.tuples.len());
        if start >= end {
            return Vec::new();
        }
        self.tuples[start..end]
            .iter()
            .filter_map(|(_, ch)| ch.clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct WordExtractor {
    pub options: TextOptions,
}

impl WordExtractor {
    pub fn new(options: TextOptions) -> Self {
        Self { options }
    }

    pub fn extract_wordmap(&self, chars: &[Char], return_chars: bool) -> WordMap {
        let mut tuples = Vec::new();
        for (word, group) in self.iter_extract_tuples(chars, return_chars) {
            tuples.push((word, group));
        }
        WordMap { tuples }
    }

    pub fn extract_words(&self, chars: &[Char], return_chars: bool) -> Vec<Word> {
        self.iter_extract_tuples(chars, return_chars)
            .into_iter()
            .map(|(word, _)| word)
            .collect()
    }

    fn iter_extract_tuples(&self, chars: &[Char], return_chars: bool) -> Vec<(Word, Vec<Char>)> {
        let mut sorted = chars.to_vec();
        if !self.options.use_text_flow {
            sorted.sort_by(|a, b| {
                a.upright
                    .cmp(&b.upright)
                    .then_with(|| a.doctop.total_cmp(&b.doctop))
                    .then_with(|| a.x0.total_cmp(&b.x0))
            });
        }

        let mut groups: Vec<Vec<Char>> = Vec::new();
        for ch in sorted {
            if let Some(last_group) = groups.last_mut() {
                let same_upright = last_group.last().map(|item| item.upright == ch.upright).unwrap_or(false);
                if same_upright {
                    last_group.push(ch);
                } else {
                    groups.push(vec![ch]);
                }
            } else {
                groups.push(vec![ch]);
            }
        }

        let mut out = Vec::new();
        for group in groups {
            for (chars_in_line, direction) in self.iter_chars_to_lines(&group) {
                for word_chars in self.iter_chars_to_words(&chars_in_line, direction) {
                    let word = self.merge_chars(&word_chars, direction, return_chars);
                    out.push((word, word_chars));
                }
            }
        }
        out
    }

    fn merge_chars(&self, ordered_chars: &[Char], direction: Direction, return_chars: bool) -> Word {
        let bbox = objects_to_bbox(ordered_chars).unwrap_or_default();
        let doctop_adj = ordered_chars.first().map(|item| item.doctop - item.top).unwrap_or(0.0);
        Word {
            text: ordered_chars
                .iter()
                .map(|ch| expand_ligature_text(&ch.text, self.options.expand_ligatures))
                .collect(),
            x0: bbox.x0,
            top: bbox.top,
            x1: bbox.x1,
            bottom: bbox.bottom,
            doctop: bbox.top + doctop_adj,
            width: bbox.width(),
            height: bbox.height(),
            upright: ordered_chars.first().map(|item| item.upright).unwrap_or(true),
            direction,
            chars: if return_chars { Some(ordered_chars.to_vec()) } else { None },
        }
    }

    fn char_dir(&self, upright: bool) -> Direction {
        if upright {
            self.options.char_dir
        } else {
            self.options.resolved_char_dir_rotated()
        }
    }

    fn line_dir(&self, upright: bool) -> Direction {
        if upright {
            self.options.line_dir
        } else {
            self.options.resolved_line_dir_rotated()
        }
    }

    fn iter_chars_to_lines(&self, chars: &[Char]) -> Vec<(Vec<Char>, Direction)> {
        if chars.is_empty() {
            return Vec::new();
        }
        let upright = chars[0].upright;
        let line_dir = self.line_dir(upright);
        let char_dir = self.char_dir(upright);

        let tol = if matches!(line_dir, Direction::Ttb | Direction::Btt) {
            self.options.y_tolerance
        } else {
            self.options.x_tolerance
        };

        let mut line_groups = cluster_items(chars, |ch| line_cluster_value(ch, line_dir), tol);

        for group in &mut line_groups {
            group.sort_by(|a, b| {
                let ka = sort_key(a, char_dir);
                let kb = sort_key(b, char_dir);
                ka.0.total_cmp(&kb.0).then_with(|| ka.1.total_cmp(&kb.1))
            });
        }

        line_groups.into_iter().map(|group| (group, char_dir)).collect()
    }

    fn iter_chars_to_words(&self, ordered_chars: &[Char], direction: Direction) -> Vec<Vec<Char>> {
        let mut words: Vec<Vec<Char>> = Vec::new();
        let punctuation = self.options.split_at_punctuation.clone().unwrap_or_default();
        let mut saw_space = false;
        let mut force_new_after_punctuation = false;

        for ch in ordered_chars.iter().cloned() {
            if !self.options.keep_blank_chars && ch.text.chars().all(|c| c.is_whitespace()) {
                saw_space = true;
                force_new_after_punctuation = false;
                continue;
            }

            if !punctuation.is_empty() && ch.text.chars().all(|c| punctuation.contains(c)) {
                words.push(vec![ch]);
                saw_space = false;
                force_new_after_punctuation = true;
                continue;
            }

            let should_start_new = saw_space
                || force_new_after_punctuation
                || words
                    .last()
                    .and_then(|word| word.last())
                    .map(|prev| {
                        let x_tol = self
                            .options
                            .x_tolerance_ratio
                            .map(|ratio| ratio * prev.size)
                            .unwrap_or(self.options.x_tolerance);

                        let y_tol = self
                            .options
                            .y_tolerance_ratio
                            .map(|ratio| ratio * prev.size)
                            .unwrap_or(self.options.y_tolerance);

                        char_begins_new_word(prev, &ch, direction, x_tol, y_tol)
                    })
                    .unwrap_or(false);
            saw_space = false;
            force_new_after_punctuation = false;

            if should_start_new {
                words.push(vec![ch]);
            } else if let Some(last) = words.last_mut() {
                last.push(ch);
            } else {
                words.push(vec![ch]);
            }
        }

        words.into_iter().filter(|word| !word.is_empty()).collect()
    }
}

pub fn chars_to_textmap(chars: &[Char], options: &TextOptions) -> TextMap {
    let mut opts = options.clone();
    if opts.layout_bbox.is_none() {
        opts.layout_bbox = objects_to_bbox(chars);
    }
    if opts.layout_width.is_none() {
        if let Some(bbox) = opts.layout_bbox {
            opts.layout_width = Some(bbox.width());
        }
    }
    if opts.layout_height.is_none() {
        if let Some(bbox) = opts.layout_bbox {
            opts.layout_height = Some(bbox.height());
        }
    }

    let extractor = WordExtractor::new(opts.clone());
    extractor.extract_wordmap(chars, true).to_textmap(&opts)
}

pub fn extract_text(chars: &[Char], options: &TextOptions) -> String {
    chars_to_textmap(chars, options).as_string()
}

pub fn extract_words(chars: &[Char], options: &TextOptions, return_chars: bool) -> Vec<Word> {
    WordExtractor::new(options.clone()).extract_words(chars, return_chars)
}

pub fn extract_text_lines(chars: &[Char], options: &TextOptions, strip: bool, return_chars: bool) -> Vec<TextLine> {
    chars_to_textmap(chars, options).extract_text_lines(strip, return_chars)
}

pub fn extract_text_simple(chars: &[Char], x_tolerance: f64, y_tolerance: f64) -> String {
    let clustered = cluster_items(chars, |ch| ch.doctop, y_tolerance);
    clustered
        .into_iter()
        .map(|mut line| {
            line.sort_by(|a, b| a.x0.total_cmp(&b.x0));
            collate_line(&line, x_tolerance)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn collate_line(line_chars: &[Char], tolerance: f64) -> String {
    collate_line_with_ligatures(line_chars, tolerance, true)
}

pub fn collate_line_with_ligatures(line_chars: &[Char], tolerance: f64, expand_ligatures: bool) -> String {
    let mut line = String::with_capacity(line_chars.iter().map(|ch| ch.text.len()).sum::<usize>() + line_chars.len());
    let mut last_x1: Option<f64> = None;
    for ch in line_chars {
        if let Some(prev_x1) = last_x1 {
            if ch.x0 > prev_x1 + tolerance {
                line.push(' ');
            }
        }
        line.push_str(expand_ligature_text(&ch.text, expand_ligatures));
        last_x1 = Some(ch.x1);
    }
    line
}

pub fn dedupe_chars(chars: &[Char], options: &DedupeOptions) -> Vec<Char> {
    if chars.is_empty() {
        return Vec::new();
    }

    let mut indexed: Vec<(usize, Char)> = chars.iter().cloned().enumerate().collect();
    indexed.sort_by(|a, b| dedupe_cmp(&a.1, &b.1, &options.extra_attrs));

    let mut kept: Vec<(usize, Char)> = Vec::new();
    let mut start = 0usize;
    while start < indexed.len() {
        let mut end = start + 1;
        while end < indexed.len()
            && dedupe_same_key(&indexed[start].1, &indexed[end].1, &options.extra_attrs)
        {
            end += 1;
        }

        let group: Vec<(usize, Char)> = indexed[start..end].to_vec();
        let y_clusters = cluster_items(&group, |(_, ch)| ch.doctop, options.tolerance);
        for y_cluster in y_clusters {
            let x_clusters = cluster_items(&y_cluster, |(_, ch)| ch.x0, options.tolerance);
            for x_cluster in x_clusters {
                let mut cluster = x_cluster;
                cluster.sort_by(|a, b| {
                    a.1.doctop
                        .total_cmp(&b.1.doctop)
                        .then_with(|| a.1.x0.total_cmp(&b.1.x0))
                });
                kept.push(cluster[0].clone());
            }
        }

        start = end;
    }

    kept.sort_by(|a, b| a.0.cmp(&b.0));
    kept.into_iter().map(|(_, ch)| ch).collect()
}

fn dedupe_cmp(a: &Char, b: &Char, extra_attrs: &[String]) -> std::cmp::Ordering {
    a.upright
        .cmp(&b.upright)
        .then_with(|| a.text.cmp(&b.text))
        .then_with(|| extra_attr_cmp(a, b, extra_attrs))
        .then_with(|| a.doctop.total_cmp(&b.doctop))
        .then_with(|| a.x0.total_cmp(&b.x0))
}

fn extra_attr_cmp(a: &Char, b: &Char, extra_attrs: &[String]) -> std::cmp::Ordering {
    for attr in extra_attrs {
        let ord = match attr.as_str() {
            "object_type" => a.object_type.cmp(&b.object_type),
            "page_number" => a.page_number.cmp(&b.page_number),
            "fontname" => a.fontname.cmp(&b.fontname),
            "size" => a.size.total_cmp(&b.size),
            "adv" => a.adv.total_cmp(&b.adv),
            "width" => a.width.total_cmp(&b.width),
            "height" => a.height.total_cmp(&b.height),
            "upright" => a.upright.cmp(&b.upright),
            "mcid" => a.mcid.cmp(&b.mcid),
            "tag" => a.tag.cmp(&b.tag),
            "ncs" => a.ncs.cmp(&b.ncs),
            "stroking_color" => a.stroking_color.cmp(&b.stroking_color),
            "non_stroking_color" => a.non_stroking_color.cmp(&b.non_stroking_color),
            "matrix" => cmp_f64_slices(&a.matrix, &b.matrix),
            _ => std::cmp::Ordering::Equal,
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    std::cmp::Ordering::Equal
}

fn dedupe_same_key(a: &Char, b: &Char, extra_attrs: &[String]) -> bool {
    if a.upright != b.upright || a.text != b.text {
        return false;
    }
    extra_attr_cmp(a, b, extra_attrs) == std::cmp::Ordering::Equal
}

fn cmp_f64_slices<const N: usize>(a: &[f64; N], b: &[f64; N]) -> std::cmp::Ordering {
    for (lhs, rhs) in a.iter().zip(b.iter()) {
        let ord = lhs.total_cmp(rhs);
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    std::cmp::Ordering::Equal
}

fn expand_ligature_text(text: &str, expand_ligatures: bool) -> &str {
    if !expand_ligatures {
        return text;
    }

    match text {
        "ﬀ" => "ff",
        "ﬃ" => "ffi",
        "ﬄ" => "ffl",
        "ﬁ" => "fi",
        "ﬂ" => "fl",
        "ﬆ" => "st",
        "ﬅ" => "st",
        _ => text,
    }
}

fn byte_to_char_index(s: &str, byte_idx: usize) -> usize {
    s[..byte_idx].chars().count()
}

fn position_key_from_bbox(bbox: BBox, direction: Direction) -> f64 {
    match direction {
        Direction::Ttb => bbox.top,
        Direction::Btt => bbox.bottom,
        Direction::Ltr => bbox.x0,
        Direction::Rtl => bbox.x1,
    }
}

fn position_value<T: TextObject>(obj: &T, direction: Direction) -> f64 {
    match direction {
        Direction::Ttb => obj.top(),
        Direction::Btt => obj.bottom(),
        Direction::Ltr => obj.x0(),
        Direction::Rtl => obj.x1(),
    }
}

fn line_cluster_value<T: TextObject>(obj: &T, direction: Direction) -> f64 {
    match direction {
        Direction::Ttb => obj.top(),
        Direction::Btt => -obj.bottom(),
        Direction::Ltr => obj.x0(),
        Direction::Rtl => -obj.x1(),
    }
}

fn sort_key<T: TextObject>(obj: &T, direction: Direction) -> (f64, f64) {
    match direction {
        Direction::Ttb => (obj.top(), obj.bottom()),
        Direction::Btt => (-(obj.top() + obj.height()), -obj.top()),
        Direction::Ltr => (obj.x0(), obj.x0()),
        Direction::Rtl => (-obj.x1(), -obj.x0()),
    }
}

fn char_begins_new_word(prev: &Char, curr: &Char, direction: Direction, x_tolerance: f64, y_tolerance: f64) -> bool {
    let (ax, bx, cx, ay, cy, x, y) = match direction {
        Direction::Ltr => (
            prev.x0,
            prev.x1,
            curr.x0,
            prev.top,
            curr.top,
            x_tolerance,
            y_tolerance,
        ),
        Direction::Rtl => (
            -prev.x1,
            -prev.x0,
            -curr.x1,
            prev.top,
            curr.top,
            x_tolerance,
            y_tolerance,
        ),
        Direction::Ttb => (
            prev.top,
            prev.bottom,
            curr.top,
            prev.x0,
            curr.x0,
            y_tolerance,
            x_tolerance,
        ),
        Direction::Btt => (
            -prev.bottom,
            -prev.top,
            -curr.bottom,
            prev.x0,
            curr.x0,
            y_tolerance,
            x_tolerance,
        ),
    };

    (cx < ax) || (cx > bx + x) || (cy - ay).abs() > y
}

trait TextObject {
    fn x0(&self) -> f64;
    fn x1(&self) -> f64;
    fn top(&self) -> f64;
    fn bottom(&self) -> f64;
    fn height(&self) -> f64;
}

impl TextObject for Char {
    fn x0(&self) -> f64 { self.x0 }
    fn x1(&self) -> f64 { self.x1 }
    fn top(&self) -> f64 { self.top }
    fn bottom(&self) -> f64 { self.bottom }
    fn height(&self) -> f64 { self.height }
}

impl TextObject for Word {
    fn x0(&self) -> f64 { self.x0 }
    fn x1(&self) -> f64 { self.x1 }
    fn top(&self) -> f64 { self.top }
    fn bottom(&self) -> f64 { self.bottom }
    fn height(&self) -> f64 { self.height }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_char(text: &str, x0: f64, x1: f64) -> Char {
        Char {
            object_type: "char".to_string(),
            page_number: 1,
            text: text.to_string(),
            x0,
            top: 0.0,
            x1,
            bottom: 10.0,
            y0: 0.0,
            y1: 10.0,
            doctop: 0.0,
            width: x1 - x0,
            height: 10.0,
            size: 10.0,
            adv: x1 - x0,
            upright: true,
            fontname: "Test".to_string(),
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            mcid: None,
            tag: None,
            ncs: None,
            stroking_color: None,
            non_stroking_color: None,
        }
    }

    #[test]
    fn punctuation_splits_before_and_after_marker() {
        let chars = vec![test_char("a", 0.0, 5.0), test_char(",", 5.0, 6.0), test_char("b", 6.0, 11.0)];
        let options = TextOptions::default().with_split_at_punctuation(",");
        let words = extract_words(&chars, &options, false);
        let texts = words.into_iter().map(|word| word.text).collect::<Vec<_>>();
        assert_eq!(texts, vec!["a", ",", "b"]);
    }

    #[test]
    fn ligatures_expand_consistently_for_words_and_textmaps() {
        let chars = vec![test_char("ﬁ", 0.0, 5.0)];
        let options = TextOptions::default();
        let words = extract_words(&chars, &options, false);
        assert_eq!(words[0].text, "fi");
        assert_eq!(chars_to_textmap(&chars, &options).as_string(), "fi");
    }
}
