use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::error::MyError;
use crate::settings::Settings;

struct MarkdownPage<'a> {
    title: String,
    events: Vec<Event<'a>>,
}

fn try_get_h1_title(
    settings: &Settings,
    fallback_file_name: &str,
    events: &mut Vec<Event>,
) -> String {
    if settings.h1_title() && events.len() >= 2 {
        if let Event::Start(Tag::Heading(HeadingLevel::H1, _, _)) = events[0] {
            let end_ndx = events
                .iter()
                .position(|e| matches!(e, Event::End(Tag::Heading(HeadingLevel::H1, _, _))))
                .unwrap();
            let title = events[1..end_ndx]
                .iter()
                .map(|e| match e {
                    Event::Text(str) => str.to_string(),
                    not_str => panic!("Expected str, got: {:?}", not_str),
                })
                .collect();
            events.drain(0..=end_ndx);
            return title;
        }
    }
    fallback_file_name.to_owned()
}

impl<'a> MarkdownPage<'a> {
    fn new(settings: &'a Settings, file_name: &'a str, src: &'a str) -> MarkdownPage<'a> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        let mut events: Vec<Event<'a>> = Parser::new_ext(src, options).collect();
        let title = try_get_h1_title(settings, file_name, &mut events);

        MarkdownPage { title, events }
    }

    fn title(&'a self) -> &'a str {
        &self.title
    }

    fn render_html(self) -> String {
        let mut rendered_markdown = String::new();
        html::push_html(&mut rendered_markdown, self.events.into_iter());
        rendered_markdown
    }
}

pub struct Page {
    pub title: String,
    pub body: String,
}

// TODO: figure out how to group methods together by language. Maybe using trait objects?
#[derive(Debug, EnumIter)]
enum MarkupLanguage {
    Markdown,
}

impl MarkupLanguage {
    fn file_extensions(&self) -> &[&str] {
        match self {
            MarkupLanguage::Markdown => {
                static MARKDOWN_EXTENSIONS: &[&str] = &["md"];
                MARKDOWN_EXTENSIONS
            }
        }
    }

    fn render(
        &self,
        file_stem: &str,
        file_contents: &str,
        settings: &Settings,
    ) -> Result<Page, MyError> {
        match self {
            MarkupLanguage::Markdown => {
                let markdown_page = MarkdownPage::new(settings, file_stem, file_contents);

                let title = markdown_page.title().to_owned();
                let body = markdown_page.render_html();

                Ok(Page { title, body })
            }
        }
    }

    fn raw(
        &self,
        file_stem: &str,
        file_contents: &str,
        settings: &Settings,
    ) -> Result<Page, MyError> {
        match self {
            MarkupLanguage::Markdown => {
                let markdown_page = MarkdownPage::new(settings, file_stem, file_contents);

                let title = markdown_page.title().to_owned();

                Ok(Page {
                    title,
                    body: file_contents.to_owned(),
                })
            }
        }
    }
}

fn get_language_for_file_extension(file_extension: &str) -> Option<MarkupLanguage> {
    for lang in MarkupLanguage::iter() {
        for ext in lang.file_extensions() {
            if file_extension == *ext {
                return Some(lang);
            }
        }
    }
    None
}

/// Gets content of page, rendered as HTML.
pub fn get_page(
    file_stem: &str,
    file_extension: &str,
    bytes: &[u8],
    settings: &Settings,
) -> Result<Option<Page>, MyError> {
    match get_language_for_file_extension(file_extension) {
        None => Ok(None),
        Some(lang) => Ok(Some(lang.render(
            file_stem,
            std::str::from_utf8(bytes)?,
            settings,
        )?)),
    }
}

/// Gets raw contents of page, after processing metadata
pub fn get_raw_page(
    file_stem: &str,
    file_extension: &str,
    bytes: &[u8],
    settings: &Settings,
) -> Result<Option<Page>, MyError> {
    match get_language_for_file_extension(file_extension) {
        None => Ok(None),
        Some(lang) => Ok(Some(lang.raw(
            file_stem,
            std::str::from_utf8(bytes)?,
            settings,
        )?)),
    }
}

pub fn is_page(file_extension: &str) -> bool {
    get_language_for_file_extension(file_extension).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_title() {
        let settings = Settings::new("Home", false);
        let input = "# First H1\n# Second H1";
        let markdown_page = MarkdownPage::new(&settings, "file_name", &input);
        assert_eq!("file_name", markdown_page.title());
        let rendered = markdown_page.render_html();
        assert_eq!("<h1>First H1</h1>\n<h1>Second H1</h1>\n", rendered);
    }

    #[test]
    fn test_h1_title() {
        let settings = Settings::new("Home", true);
        let input = "# First H1\n# Second H1";
        let markdown_page = MarkdownPage::new(&settings, "file_name", &input);
        assert_eq!("First H1", markdown_page.title());
        let rendered = markdown_page.render_html();
        assert_eq!("<h1>Second H1</h1>\n", rendered);
    }

    #[test]
    fn test_h1_title_complicated() {
        let settings = Settings::new("Home", true);
        let input = "# Austin\'s Wiki\nwords words words";
        let markdown_page = MarkdownPage::new(&settings, "file_name", &input);
        assert_eq!("Austin\u{2019}s Wiki", markdown_page.title());
        let rendered = markdown_page.render_html();
        assert_eq!("<p>words words words</p>\n", rendered);
    }
}
