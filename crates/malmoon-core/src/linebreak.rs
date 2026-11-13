//! Line breaking that never splits a word.
//!
//! Korean is written with spaces between words (어절), and a line that breaks inside one reads
//! badly. Some renderers break Korean between any two syllables, so captions are split into lines
//! here and handed over one line at a time.

/// Splits `text` into lines no wider than `max_width`, breaking only at whitespace.
///
/// `measure` returns the rendered width of a string in the same unit as `max_width`. A single
/// word wider than `max_width` gets a line of its own rather than being split.
pub fn wrap_words(text: &str, max_width: f32, mut measure: impl FnMut(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();

    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
            continue;
        }
        let candidate_len = line.len();
        line.push(' ');
        line.push_str(word);
        if measure(&line) > max_width {
            line.truncate(candidate_len);
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character is one unit wide.
    fn chars(text: &str) -> f32 {
        text.chars().count() as f32
    }

    #[test]
    fn korean_breaks_only_between_words() {
        let lines = wrap_words("상대방이 강조한 내용을 정리해 드리겠습니다", 9.0, chars);
        assert_eq!(lines, ["상대방이 강조한", "내용을 정리해", "드리겠습니다"]);
    }

    #[test]
    fn a_word_wider_than_the_line_is_kept_whole() {
        let lines = wrap_words("a incomprehensibilities b", 5.0, chars);
        assert_eq!(lines, ["a", "incomprehensibilities", "b"]);
    }

    #[test]
    fn whitespace_runs_collapse_and_empty_input_yields_no_lines() {
        assert_eq!(
            wrap_words("  hello   world  ", 20.0, chars),
            ["hello world"]
        );
        assert!(wrap_words("   ", 20.0, chars).is_empty());
    }

    #[test]
    fn text_that_fits_stays_on_one_line() {
        assert_eq!(wrap_words("짧은 문장", 20.0, chars), ["짧은 문장"]);
    }
}
