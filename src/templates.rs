use askama::Template;

struct Breadcrumb<'a> {
    name: &'a str,
    href: String,
}

impl<'a> Breadcrumb<'a> {
    fn from_elements(path_elements: &'a [String]) -> Vec<Self> {
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
        breadcrumbs
    }
}

#[derive(Template)]
#[template(path = "view_page.html", escape = "none")]
struct ViewPageTemplate<'a> {
    title: &'a str,
    content: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
}

pub fn render_page(title: &str, content: &str, path_elements: &[String]) -> askama::Result<String> {
    let breadcrumbs = Breadcrumb::from_elements(path_elements);
    let page = ViewPageTemplate {
        title,
        content,
        breadcrumbs,
    };
    // TODO: render into a stream directly instead of crating this String.
    page.render()
}

#[derive(Template)]
#[template(path = "page_placeholder.html")]
struct PagePlaceholderTemplate<'a> {
    title: &'a str,
    file_path: &'a str,
    create_url: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
}

pub fn render_page_placeholder(
    title: &str,
    file_path: &str,
    create_url: &str,
    path_elements: &[String],
) -> askama::Result<String> {
    let breadcrumbs = Breadcrumb::from_elements(path_elements);
    let template = PagePlaceholderTemplate {
        title,
        file_path,
        create_url,
        breadcrumbs,
    };
    template.render()
}
