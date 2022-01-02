use askama::Template;

#[derive(Template)]
#[template(path = "layout.html", escape="none")]
struct LayoutTemplate<'a> {
    title: &'a str,
    content: &'a str,
}

pub fn render_page(title: &str, content: &str) -> askama::Result<String> {
    let page = LayoutTemplate {
        title, content
    };
    // TODO: render into a stream directly instead of crating this String.
    page.render()
}