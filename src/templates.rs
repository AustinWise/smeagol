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
    edit_url: &'a str,
    content: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
}

pub fn render_page(
    title: &str,
    edit_url: &str,
    content: &str,
    path_elements: &[String],
) -> askama::Result<String> {
    let breadcrumbs = Breadcrumb::from_elements(path_elements);
    let page = ViewPageTemplate {
        title,
        edit_url,
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

//NOTE: this MUST escape the content so it displays correctly in the textarea
#[derive(Template)]
#[template(path = "edit_page.html")]
struct EditTemplate<'a> {
    title: &'a str,
    post_url: &'a str,
    view_url: &'a str,
    content: &'a str,
    breadcrumbs: Vec<Breadcrumb<'a>>,
}

pub fn render_edit_page(
    title: &str,
    post_url: &str,
    view_url: &str,
    content: &str,
    path_elements: &[String],
) -> askama::Result<String> {
    let breadcrumbs = Breadcrumb::from_elements(path_elements);
    let template = EditTemplate {
        title,
        post_url,
        view_url,
        content,
        breadcrumbs,
    };
    template.render()
}
