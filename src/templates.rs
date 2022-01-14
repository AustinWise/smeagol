use askama::Template;

struct Breadcrumb<'a> {
    name: &'a str,
    href: String,
}

#[derive(Template)]
#[template(path = "layout.html", escape = "none")]
struct LayoutTemplate<'a> {
    title: &'a str,
    content: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
}

pub fn render_page(title: &str, content: &str, path_elements: &[String]) -> askama::Result<String> {
    let mut breadcrumbs = Vec::with_capacity(path_elements.len());
    let mut href: String = "/".into();
    for el in path_elements {
        href += el;
        href += "/";
        breadcrumbs.push(Breadcrumb {
            name: el,
            href: href.clone(),
        });
    }
    let page = LayoutTemplate {
        title,
        content,
        breadcrumbs,
    };
    // TODO: render into a stream directly instead of crating this String.
    page.render()
}
