pub fn sanitize_name(name: String) -> String {
    // See <https://stackoverflow.com/questions/7381974/which-characters-need-to-be-escaped-in-html#7382028>
    name.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

#[test]
fn simple_xss_gets_replaced() {
    assert_eq!(
        sanitize_name("<script>alert(1);</script>".to_string()),
        "&lt;script&gt;alert(1);&lt;/script&gt;".to_string()
    );
}

#[test]
fn all_evil_chars_get_replaced() {
    assert_eq!(
        sanitize_name("&<>\"'".to_string()),
        "&amp;&lt;&gt;&quot;&#39;".to_string()
    )
}
