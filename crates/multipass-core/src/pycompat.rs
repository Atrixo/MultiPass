use regex::Regex;

pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

pub fn lstrip(s: &str) -> &str {
    s.trim_start_matches(is_space)
}

pub fn rstrip(s: &str) -> &str {
    s.trim_end_matches(is_space)
}

fn is_line_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

pub fn splitlines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if is_line_break(c) {
            out.push(&s[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' {
                if let Some(&(j, '\n')) = it.peek() {
                    it.next();
                    end = j + 1;
                }
            }
            start = end;
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

pub fn fmt_num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0');
    if s == "-0." || s == "-0" || s == "0" {
        "0.".to_string()
    } else {
        s.to_string()
    }
}

pub fn regex(pattern: &str) -> Regex {
    Regex::new(&pattern.replace(r"\s", r"[\s\x1C-\x1F]")).expect("gueltiger Ausdruck")
}

pub fn parse_uint(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

pub fn parse_float(s: &str) -> Option<f64> {
    if !s.is_ascii() {
        return None;
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitlines_like_python() {
        assert_eq!(splitlines(""), Vec::<&str>::new());
        assert_eq!(splitlines("\n"), vec![""]);
        assert_eq!(splitlines("a\r\nb\n"), vec!["a", "b"]);
        assert_eq!(splitlines("a\n\nb"), vec!["a", "", "b"]);
        assert_eq!(splitlines("a\rb\r\n\r\nc"), vec!["a", "b", "", "c"]);
        assert_eq!(splitlines("a\x0bb\x0cc\x1cd\u{85}e\u{2028}f"), vec!["a", "b", "c", "d", "e", "f"]);
        assert_eq!(splitlines("a\r"), vec!["a"]);
    }

    #[test]
    fn number_format_like_python() {
        let cases = [
            (0.03125, "0.0312"),
            (1.00005, "1.0001"),
            (2.5, "2.5"),
            (0.00005, "0.0001"),
            (-0.00004, "0."),
            (1.23455, "1.2346"),
            (15.0, "15."),
            (-0.0, "0."),
            (0.0, "0."),
            (123456.78905, "123456.7891"),
            (0.1, "0.1"),
            (1e-5, "0."),
            (4.56789, "4.5679"),
            (0.00015, "0.0001"),
            (0.00025, "0.0003"),
            (1.99995, "1.9999"),
            (7.77775, "7.7778"),
            (-12.5, "-12.5"),
        ];
        for (v, s) in cases {
            assert_eq!(fmt_num(v), s, "Wert {v}");
        }
    }

    #[test]
    fn rounding_like_python() {
        assert_eq!(format!("{:.1}", 123456.78905 / 1000.0), "123.5");
        assert_eq!(format!("{:.0}", 123456.78905), "123457");
        assert_eq!(format!("{:.0}", 2.5), "2");
        assert_eq!(format!("{:.0}", -0.0), "-0");
    }

    #[test]
    fn whitespace_like_python() {
        assert_eq!(strip("\x1f a \u{3000}"), "a");
        assert_eq!(lstrip("  G1"), "G1");
        assert_eq!(rstrip("X1 \t"), "X1");
    }

    #[test]
    fn parse_numbers() {
        assert_eq!(parse_float("1."), Some(1.0));
        assert_eq!(parse_float(".5"), Some(0.5));
        assert_eq!(parse_float("+3"), Some(3.0));
        assert_eq!(parse_float("1.2.3"), None);
        assert_eq!(parse_uint("13"), Some(13));
        assert_eq!(parse_uint("\u{0663}"), None);
    }
}
