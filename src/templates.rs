use std::ops::Deref;

use askama::Template;

use crate::assets::favicon_png_uri;
use crate::assets::primer_css_uri;
use crate::wiki::SearchResult;

use shadow_rs::shadow;

shadow!(build);

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SHORT_COMMIT: &str = build::SHORT_COMMIT;

pub struct Breadcrumb<'a> {
    name: &'a str,
    href: String,
}

impl<'a> Breadcrumb<'a> {
    pub fn new(name: &'a str, href: String) -> Self {
        Self { name, href }
    }
}

#[derive(Template)]
#[template(path = "layout.html")]
struct LayoutTemplate<'a> {
    // TODO: reduce copying here
    primer_css_uri: String,
    favicon_png_uri: String,
    title: String,
    breadcrumbs: Vec<Breadcrumb<'a>>,
    chat_url: Option<&'a str>,
    overview_url: String,
    version: &'static str,
    short_sha: &'static str,
}

impl<'a> LayoutTemplate<'a> {
    fn new_with_chat(
        title: &'a str,
        overview_url: &'a str,
        breadcrumbs: Vec<Breadcrumb<'a>>,
        chat_url: Option<&'a str>,
    ) -> Self {
        let primer_css_uri = primer_css_uri();
        let favicon_png_uri = favicon_png_uri();
        Self {
            breadcrumbs,
            favicon_png_uri,
            primer_css_uri,
            overview_url: overview_url.to_owned(),
            chat_url,
            title: title.to_owned(),
            version: VERSION,
            short_sha: SHORT_COMMIT,
        }
    }

    fn new(title: &'a str, overview_url: &'a str, breadcrumbs: Vec<Breadcrumb<'a>>) -> Self {
        Self::new_with_chat(title, overview_url, breadcrumbs, None)
    }
}

#[derive(Template)]
#[template(path = "view_page.html", escape = "none")]
struct ViewPageTemplate<'a> {
    layout: &'a LayoutTemplate<'a>,
    edit_url: &'a str,
    content: &'a str,
}

impl<'a> Deref for ViewPageTemplate<'a> {
    type Target = LayoutTemplate<'a>;

    fn deref(&self) -> &Self::Target {
        self.layout
    }
}

pub fn render_page(
    title: &str,
    edit_url: &str,
    overview_url: &str,
    chat_url: Option<&str>,
    content: &str,
    breadcrumbs: Vec<Breadcrumb<'_>>,
) -> askama::Result<String> {
    let layout = LayoutTemplate::new_with_chat(title, overview_url, breadcrumbs, chat_url);
    let page = ViewPageTemplate {
        layout: &layout,
        edit_url,
        content,
    };
    page.render()
}

#[derive(Template)]
#[template(path = "page_placeholder.html")]
struct PagePlaceholderTemplate<'a> {
    layout: &'a LayoutTemplate<'a>,
    file_path: &'a str,
    create_url: &'a str,
}

impl<'a> Deref for PagePlaceholderTemplate<'a> {
    type Target = LayoutTemplate<'a>;

    fn deref(&self) -> &Self::Target {
        self.layout
    }
}

pub fn render_page_placeholder(
    title: &str,
    file_path: &str,
    create_url: &str,
    overview_url: &str,
    breadcrumbs: Vec<Breadcrumb<'_>>,
) -> askama::Result<String> {
    let layout = LayoutTemplate::new(title, overview_url, breadcrumbs);
    let template = PagePlaceholderTemplate {
        layout: &layout,
        file_path,
        create_url,
    };
    template.render()
}

//NOTE: this MUST escape the content so it displays correctly in the textarea
#[derive(Template)]
#[template(path = "edit_page.html")]
struct EditTemplate<'a> {
    layout: &'a LayoutTemplate<'a>,
    post_url: &'a str,
    view_url: &'a str,
    preview_url: &'a str,
    message_placeholder: Option<String>,
    content: &'a str,
    authenticity_token: &'a str,
}

impl<'a> Deref for EditTemplate<'a> {
    type Target = LayoutTemplate<'a>;

    fn deref(&self) -> &Self::Target {
        self.layout
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_edit_page(
    title: &str,
    post_url: &str,
    view_url: &str,
    preview_url: &str,
    message_placeholder: Option<String>,
    content: &str,
    breadcrumbs: Vec<Breadcrumb<'_>>,
    authenticity_token: &str,
) -> askama::Result<String> {
    let layout = LayoutTemplate::new(title, "/overview", breadcrumbs);
    let template = EditTemplate {
        layout: &layout,
        post_url,
        view_url,
        preview_url,
        message_placeholder,
        content,
        authenticity_token,
    };
    template.render()
}

pub struct DirectoryEntry<'a> {
    name: &'a str,
    href: String,
}

impl<'a> DirectoryEntry<'a> {
    pub fn new(name: &'a str, href: String) -> Self {
        DirectoryEntry { name, href }
    }
}

#[derive(Template)]
#[template(path = "overview.html", escape = "none")]
struct OverviewTemplate<'a> {
    layout: &'a LayoutTemplate<'a>,
    file_svg: &'a str,
    file_directory_svg: &'a str,
    directories: Vec<DirectoryEntry<'a>>,
    files: Vec<DirectoryEntry<'a>>,
}

impl<'a> Deref for OverviewTemplate<'a> {
    type Target = LayoutTemplate<'a>;

    fn deref(&self) -> &Self::Target {
        self.layout
    }
}

pub fn render_overview(
    title: &str,
    breadcrumbs: Vec<Breadcrumb<'_>>,
    chat_url: Option<&str>,
    directories: Vec<DirectoryEntry<'_>>,
    files: Vec<DirectoryEntry<'_>>,
) -> askama::Result<String> {
    let layout = LayoutTemplate::new_with_chat(title, "/overview", breadcrumbs, chat_url);
    let file_svg = include_str!("../static/file.svg");
    let file_directory_svg = include_str!("../static/file_directory.svg");
    let template = OverviewTemplate {
        layout: &layout,
        file_svg,
        file_directory_svg,
        directories,
        files,
    };
    template.render()
}

#[derive(Template)]
#[template(path = "search_results.html", escape = "none")]
struct SearchResultsTemplate<'a> {
    layout: &'a LayoutTemplate<'a>,
    query: &'a str,
    documents: Vec<SearchResult>,
    prev_url: Option<String>,
    next_url: Option<String>,
}

impl<'a> Deref for SearchResultsTemplate<'a> {
    type Target = LayoutTemplate<'a>;

    fn deref(&self) -> &Self::Target {
        self.layout
    }
}

pub fn render_search_results(
    query: &str,
    documents: Vec<SearchResult>,
    prev_url: Option<String>,
    next_url: Option<String>,
) -> askama::Result<String> {
    let breadcrumbs = vec![];
    let layout = LayoutTemplate::new("Search results", "/overview", breadcrumbs);
    let template = SearchResultsTemplate {
        layout: &layout,
        query,
        documents,
        prev_url,
        next_url,
    };
    template.render()
}

#[derive(Template)]
#[template(path = "chat.html")]
struct ChatTemplate<'a> {
    layout: &'a LayoutTemplate<'a>,
    context: &'a str,
}

impl<'a> Deref for ChatTemplate<'a> {
    type Target = LayoutTemplate<'a>;

    fn deref(&self) -> &Self::Target {
        self.layout
    }
}

pub fn render_chat(breadcrumbs: Vec<Breadcrumb<'_>>, context: &str) -> askama::Result<String> {
    let layout = LayoutTemplate::new("Chat", "/overview", breadcrumbs);
    let template = ChatTemplate {
        layout: &layout,
        context,
    };
    template.render()
}
