//! Name transformation helpers (`snake_case` / `PascalCase` / `kebab-case` + pluralization).

/// Split a name into words on `_`, `-`, whitespace, then tokenize each chunk
/// the way Python's `re.findall(r"[A-Z]+(?=[A-Z][a-z])|[A-Z]?[a-z]+|[A-Z]+|\d+")` would.
fn split(name: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for chunk in name.split(|c: char| c == '_' || c == '-' || c.is_whitespace()) {
        if chunk.is_empty() {
            continue;
        }
        parts.extend(tokenize(chunk));
    }
    parts
}

/// Tokenize one chunk into words: uppercase runs, lowercase runs, digit runs,
/// with "HTTPServer"-style boundaries split as HTTP + Server.
fn tokenize(chunk: &str) -> Vec<String> {
    let chars: Vec<char> = chunk.chars().collect();
    let n = chars.len();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else if c.is_ascii_uppercase() {
            let start = i;
            while i < n && chars[i].is_ascii_uppercase() {
                i += 1;
            }
            if i < n && chars[i].is_ascii_lowercase() {
                if i - start == 1 {
                    // [A-Z]?[a-z]+ : single uppercase joins the following lowercase run.
                    while i < n && chars[i].is_ascii_lowercase() {
                        i += 1;
                    }
                    out.push(chars[start..i].iter().collect());
                } else {
                    // [A-Z]+(?=[A-Z][a-z]) : all but the last uppercase start the next word.
                    out.push(chars[start..i - 1].iter().collect());
                    i -= 1;
                }
            } else {
                // [A-Z]+ not followed by a lowercase letter.
                out.push(chars[start..i].iter().collect());
            }
        } else if c.is_ascii_lowercase() {
            let start = i;
            while i < n && chars[i].is_ascii_lowercase() {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else {
            // Characters no alternative matches are skipped, like re.findall does.
            i += 1;
        }
    }
    out
}

/// Python's `str.capitalize()`: first character upper, the rest lower.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out: String = first.to_uppercase().collect();
            out.extend(chars.flat_map(char::to_lowercase));
            out
        }
        None => String::new(),
    }
}

/// `UserProfile` -> `user_profile`.
#[must_use]
pub fn to_snake(name: &str) -> String {
    join_lower(&split(name), "_")
}

/// `user_profile` -> `UserProfile`.
#[must_use]
pub fn to_pascal(name: &str) -> String {
    split(name).iter().map(|p| capitalize(p)).collect()
}

/// `my-cool_app` -> `My Cool App`.
#[must_use]
pub fn to_title(name: &str) -> String {
    split(name)
        .iter()
        .map(|p| capitalize(p))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `UserProfile` -> `user-profile`.
#[must_use]
pub fn to_kebab(name: &str) -> String {
    join_lower(&split(name), "-")
}

fn join_lower(words: &[String], sep: &str) -> String {
    words
        .iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join(sep)
}

/// Simple English pluralization.
#[must_use]
pub fn to_plural(word: &str) -> String {
    let count = word.chars().count();
    if word.ends_with(['s', 'x', 'z']) || word.ends_with("ch") || word.ends_with("sh") {
        format!("{word}es")
    } else if word.ends_with('y')
        && count > 1
        && word
            .chars()
            .rev()
            .nth(1)
            .is_some_and(|c| !"aeiou".contains(c))
    {
        let trimmed: String = word.chars().take(count - 1).collect();
        format!("{trimmed}ies")
    } else if word.ends_with('f') {
        let trimmed: String = word.chars().take(count - 1).collect();
        format!("{trimmed}ves")
    } else if word.ends_with("fe") {
        let trimmed: String = word.chars().take(count - 2).collect();
        format!("{trimmed}ves")
    } else {
        format!("{word}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake() {
        assert_eq!(to_snake("UserProfile"), "user_profile");
        assert_eq!(to_snake("user-profile"), "user_profile");
        assert_eq!(to_snake("order item"), "order_item");
        assert_eq!(to_snake("user"), "user");
    }

    #[test]
    fn test_to_pascal() {
        assert_eq!(to_pascal("user_profile"), "UserProfile");
        assert_eq!(to_pascal("blog-post"), "BlogPost");
        assert_eq!(to_pascal("HTTPServer"), "HttpServer");
    }

    #[test]
    fn test_to_title() {
        assert_eq!(to_title("my-cool_app"), "My Cool App");
    }

    #[test]
    fn test_to_kebab() {
        assert_eq!(to_kebab("UserProfile"), "user-profile");
    }

    #[test]
    fn test_to_plural() {
        assert_eq!(to_plural("user"), "users");
        assert_eq!(to_plural("category"), "categories");
        assert_eq!(to_plural("box"), "boxes");
        assert_eq!(to_plural("match"), "matches");
    }
}
