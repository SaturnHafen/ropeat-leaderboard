pub fn slow_equals(a: &[u8], b: &[u8]) -> bool {
    let mut result = a.len() == b.len();

    if result {
        for (a_i, b_i) in a.iter().zip(b) {
            result &= a_i == b_i;
        }
    }

    result
}

#[test]
fn unequal_length_gets_rejected() {
    assert!(!slow_equals("abcd".as_bytes(), "abcdefgh".as_bytes()));
    assert!(!slow_equals("abcdefgh".as_bytes(), "abcd".as_bytes()));
}

#[test]
fn unequal_values_get_rejected() {
    assert!(!slow_equals("abcd".as_bytes(), "efgh".as_bytes()))
}

#[test]
fn equal_values_get_accepted() {
    assert!(slow_equals("abcd".as_bytes(), "abcd".as_bytes()))
}

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
