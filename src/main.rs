use pulldown_cmark::{html, Options, Parser};

fn main() {
    let markdown_input = "Hello world, this is a ~~complicated~~ *very simple* example.";

    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown_input, options);

    // Write to String buffer.
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    println!("{}", html_output);
}
