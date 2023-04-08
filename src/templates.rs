use askama::Template;

use crate::assets::favicon_png_uri;
use crate::assets::primer_css_uri;
use crate::wiki::SearchResult;

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
#[template(path = "view_page.html", escape = "none")]
struct ViewPageTemplate<'a> {
    primer_css_uri: &'a str,
    favicon_png_uri: &'a str,
    title: &'a str,
    edit_url: &'a str,
    overview_url: &'a str,
    content: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
}

pub fn render_page(
    title: &str,
    edit_url: &str,
    overview_url: &str,
    content: &str,
    breadcrumbs: Vec<Breadcrumb<'_>>,
) -> askama::Result<String> {
    let primer_css_uri = &primer_css_uri();
    let favicon_png_uri = &favicon_png_uri();
    let page = ViewPageTemplate {
        primer_css_uri,
        favicon_png_uri,
        title,
        edit_url,
        overview_url,
        content,
        breadcrumbs,
    };
    // TODO: render into a stream directly instead of crating this String.
    page.render()
}

#[derive(Template)]
#[template(path = "page_placeholder.html")]
struct PagePlaceholderTemplate<'a> {
    primer_css_uri: &'a str,
    favicon_png_uri: &'a str,
    title: &'a str,
    file_path: &'a str,
    create_url: &'a str,
    overview_url: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
}

pub fn render_page_placeholder(
    title: &str,
    file_path: &str,
    create_url: &str,
    overview_url: &str,
    breadcrumbs: Vec<Breadcrumb<'_>>,
) -> askama::Result<String> {
    let primer_css_uri = &primer_css_uri();
    let favicon_png_uri = &favicon_png_uri();
    let template = PagePlaceholderTemplate {
        primer_css_uri,
        favicon_png_uri,
        title,
        file_path,
        create_url,
        overview_url,
        breadcrumbs,
    };
    template.render()
}

//NOTE: this MUST escape the content so it displays correctly in the textarea
#[derive(Template)]
#[template(path = "edit_page.html")]
struct EditTemplate<'a> {
    primer_css_uri: &'a str,
    favicon_png_uri: &'a str,
    title: &'a str,
    post_url: &'a str,
    view_url: &'a str,
    preview_url: &'a str,
    overview_url: &'a str,
    message_placeholder: Option<String>,
    content: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
    authenticity_token: &'a str,
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
    let primer_css_uri = &primer_css_uri();
    let favicon_png_uri = &favicon_png_uri();
    let template = EditTemplate {
        primer_css_uri,
        favicon_png_uri,
        title,
        post_url,
        view_url,
        preview_url,
        overview_url: "/overview",
        message_placeholder,
        content,
        breadcrumbs,
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
    primer_css_uri: &'a str,
    favicon_png_uri: &'a str,
    file_svg: &'a str,
    file_directory_svg: &'a str,
    title: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
    directories: Vec<DirectoryEntry<'a>>,
    files: Vec<DirectoryEntry<'a>>,
    overview_url: &'a str,
}

pub fn render_overview(
    title: &str,
    breadcrumbs: Vec<Breadcrumb<'_>>,
    directories: Vec<DirectoryEntry<'_>>,
    files: Vec<DirectoryEntry<'_>>,
) -> askama::Result<String> {
    let primer_css_uri = &primer_css_uri();
    let favicon_png_uri = &favicon_png_uri();
    let file_svg = include_str!("../static/file.svg");
    let file_directory_svg = include_str!("../static/file_directory.svg");
    let template = OverviewTemplate {
        primer_css_uri,
        favicon_png_uri,
        file_svg,
        file_directory_svg,
        title,
        breadcrumbs,
        directories,
        files,
        overview_url: "/overview",
    };
    template.render()
}

#[derive(Template)]
#[template(path = "search_results.html", escape = "none")]
struct SearchResultsTemplate<'a> {
    primer_css_uri: &'a str,
    favicon_png_uri: &'a str,
    title: &'a str,
    query: &'a str,
    documents: Vec<SearchResult>,
    breadcrumbs: Vec<Breadcrumb<'a>>,
    prev_url: Option<String>,
    next_url: Option<String>,
    overview_url: &'a str,
}

pub fn render_search_results(
    query: &str,
    documents: Vec<SearchResult>,
    prev_url: Option<String>,
    next_url: Option<String>,
) -> askama::Result<String> {
    let primer_css_uri = &primer_css_uri();
    let favicon_png_uri = &favicon_png_uri();
    let breadcrumbs = vec![];
    let template = SearchResultsTemplate {
        primer_css_uri,
        favicon_png_uri,
        title: "Search results",
        query,
        breadcrumbs,
        documents,
        prev_url,
        next_url,
        overview_url: "/overview",
    };
    template.render()
}
