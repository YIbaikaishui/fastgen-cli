"""Tests for name transformation helpers."""

from fastgen.naming import to_kebab, to_pascal, to_plural, to_snake, to_title


def test_to_snake() -> None:
    assert to_snake("UserProfile") == "user_profile"
    assert to_snake("user-profile") == "user_profile"
    assert to_snake("order item") == "order_item"


def test_to_pascal() -> None:
    assert to_pascal("user_profile") == "UserProfile"
    assert to_pascal("blog-post") == "BlogPost"


def test_to_title() -> None:
    assert to_title("my-cool_app") == "My Cool App"


def test_to_kebab() -> None:
    assert to_kebab("UserProfile") == "user-profile"


def test_to_plural() -> None:
    assert to_plural("user") == "users"
    assert to_plural("category") == "categories"
    assert to_plural("box") == "boxes"
    assert to_plural("match") == "matches"
