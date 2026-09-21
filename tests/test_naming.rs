//! Tests for name transformation helpers.

use fastgen_cli::naming::{to_kebab, to_pascal, to_plural, to_snake, to_title};

#[test]
fn test_to_snake() {
    assert_eq!(to_snake("UserProfile"), "user_profile");
    assert_eq!(to_snake("user-profile"), "user_profile");
    assert_eq!(to_snake("order item"), "order_item");
}

#[test]
fn test_to_pascal() {
    assert_eq!(to_pascal("user_profile"), "UserProfile");
    assert_eq!(to_pascal("blog-post"), "BlogPost");
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
