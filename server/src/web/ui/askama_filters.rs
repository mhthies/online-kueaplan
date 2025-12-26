/// Convert a Markdown string to HTML
///
/// This filter is based on the comrak Markdown parser (https://docs.rs/comrak/latest/comrak/).
/// Most of the comrak GFM extensions are enabled.
/// In addition, we increase the heading level of all Markdown headings by 3, i.e. h1 becomes h4,
/// h2 becomes h5 and all additional headings become h6.
#[askama::filter_fn]
pub fn markdown(
    input: &str,
    _: &dyn askama::Values,
) -> askama::Result<askama::filters::Safe<String>> {
    let arena = comrak::Arena::new();
    let options = comrak::options::Options {
        extension: comrak::options::Extension::builder()
            .strikethrough(true)
            .tagfilter(true)
            .table(true)
            .footnotes(true)
            .underline(true)
            .build(),
        parse: Default::default(),
        render: comrak::options::Render::builder().escape(true).build(),
    };
    let ast_root = comrak::parse_document(&arena, input, &options);

    markdown_increase_heading_level(ast_root, 3);

    let mut html = String::new();
    comrak::format_html(ast_root, &options, &mut html)?;
    Ok(askama::filters::Safe(html))
}

/// Helper function to increase the heading level of all headings in the given comrak AST.
///
/// Params
/// ======
/// * `ast_root` root of the comrak abstract syntax tree, parsed from the markdown document. The
///   heading nodes of the AST are changed in-place.
/// * `increase_by` amount of levels to add to each heading. I.e. with `increase_by = 2`, h1 becomes
///   h3 and h2 becomes h4 and so on.
fn markdown_increase_heading_level<'a>(ast_root: &'a comrak::nodes::AstNode<'a>, increase_by: u8) {
    for node in ast_root.descendants() {
        if let comrak::nodes::NodeValue::Heading(ref mut heading) = node.data.borrow_mut().value {
            heading.level = (heading.level + increase_by).clamp(1, 6);
        }
    }
}

/// Shorten a text to the given `length` by replacing any additional characters with an ellipsis
/// character ("…").
#[askama::filter_fn]
pub fn ellipsis(value: &str, _: &dyn askama::Values, length: usize) -> askama::Result<String> {
    if value.chars().count() > length {
        Ok(format!(
            "{}…",
            value.chars().take(length - 1).collect::<String>()
        ))
    } else {
        Ok(value.to_owned())
    }
}
