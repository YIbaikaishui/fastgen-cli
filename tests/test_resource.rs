//! Tests for `fastgen make resource`: the `--fields` spec parser and the
//! generated CRUD slice.

use std::path::Path;

use fastgen_cli::fields::{parse_fields, FieldType, Fields};
use fastgen_cli::generators::resource::generate_resource;

/// A temp dir that looks like a real `src`-layout project, so `source_dir_name`
/// detection resolves to `src`.
fn project() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("src/main.py"), "# app\n").unwrap();
    tmp
}

fn generate(name: &str, fields: Option<&str>) -> (tempfile::TempDir, Vec<std::path::PathBuf>) {
    let tmp = project();
    let files = generate_resource(name, tmp.path(), None, fields, false, false).unwrap();
    let paths: Vec<std::path::PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    (tmp, paths)
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"))
}

// ---------------------------------------------------------------- spec parsing

#[test]
fn parses_a_full_spec() {
    let fields = parse_fields("title:str, price:decimal, active:bool?, due:date").unwrap();
    assert_eq!(fields.items.len(), 4);
    assert_eq!(fields.items[0].name, "title");
    assert_eq!(fields.items[0].ty, FieldType::Str);
    assert!(!fields.items[0].optional);
    assert_eq!(fields.items[1].ty, FieldType::Decimal);
    assert!(
        fields.items[2].optional,
        "a `?` suffix marks the field optional"
    );
    assert_eq!(fields.items[3].ty, FieldType::Date);
}

#[test]
fn tolerates_spacing_and_casing() {
    let fields = parse_fields("  Title : STR ,  Price:FLOAT ").unwrap();
    assert_eq!(
        fields.items[0].name, "title",
        "names are normalised to snake_case"
    );
    assert_eq!(fields.items[1].name, "price");
    assert_eq!(fields.items[1].ty, FieldType::Float);
}

#[test]
fn rejects_bad_specs() {
    for spec in [
        "a:wat",       // unknown type
        "abc",         // no colon
        "a:str,A:str", // duplicate after normalisation
        "1a:str",      // not an identifier
        "a:str,",      // trailing comma is fine, so this one must parse
    ] {
        if spec == "a:str," {
            assert!(parse_fields(spec).is_ok(), "{spec} should parse");
            continue;
        }
        let err = parse_fields(spec).unwrap_err().to_string();
        assert!(!err.is_empty(), "{spec} should fail with a message");
    }
    assert!(
        parse_fields("  ,  ").is_err(),
        "an all-blank spec is an error"
    );
    assert!(parse_fields("").is_err(), "an empty spec is an error");
}

#[test]
fn every_type_maps_to_a_column() {
    for name in FieldType::NAMES {
        let fields = parse_fields(&format!("f:{name}")).unwrap();
        let column = fields.items[0].column_expr();
        assert!(
            column.starts_with("mapped_column("),
            "{name} produced {column}"
        );
        assert!(!fields.items[0].annotation().is_empty());
    }
}

#[test]
fn optional_fields_are_nullable_and_defaulted() {
    let fields = parse_fields("a:str, b:str?").unwrap();
    assert_eq!(fields.items[0].annotation(), "str");
    assert_eq!(fields.items[0].column_expr(), "mapped_column(String(255))");
    assert_eq!(fields.items[1].annotation(), "str | None");
    assert!(
        fields.items[1].column_expr().contains("nullable=True"),
        "an optional field needs a nullable column"
    );
    assert_eq!(fields.items[1].update_annotation(), "str | None");
    assert_eq!(fields.items[0].update_annotation(), "str | None");
}

#[test]
fn uuid_columns_get_a_default() {
    let fields = parse_fields("id:uuid").unwrap();
    assert!(
        fields.items[0].column_expr().contains("default=uuid4"),
        "a required uuid must be generated, not nullable"
    );
}

#[test]
fn imports_are_deduplicated_and_sorted() {
    let fields = parse_fields("a:date, b:date, c:decimal, d:uuid").unwrap();
    assert_eq!(
        fields.imports(),
        vec![
            "from datetime import date",
            "from decimal import Decimal",
            "from uuid import UUID, uuid4"
        ]
    );
    assert_eq!(
        fields.schema_imports(),
        vec![
            "from datetime import date",
            "from decimal import Decimal",
            "from uuid import UUID"
        ],
        "the schemas never use uuid4"
    );
}

#[test]
fn fallback_field_set_is_a_single_name() {
    let fields = Fields::fallback();
    assert_eq!(fields.items.len(), 1);
    assert_eq!(fields.items[0].name, "name");
    assert_eq!(fields.items[0].ty, FieldType::Str);
    assert!(fields.first_required().is_some());
}

// ------------------------------------------------------------- generated code

#[test]
fn generates_the_full_slice() {
    let (tmp, paths) = generate("product", Some("title:str, price:decimal"));
    let names: Vec<String> = paths
        .iter()
        .map(|p| {
            p.strip_prefix(tmp.path())
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    for expected in [
        "src/modules/product/__init__.py",
        "src/modules/product/api/router.py",
        "src/modules/product/application/schemas.py",
        "src/modules/product/application/product_service.py",
        "src/modules/product/domain/model.py",
        "src/modules/product/domain/repository.py",
        "src/modules/product/infrastructure/product_repository.py",
        "src/modules/product/tests/test_product.py",
    ] {
        assert!(names.contains(&expected.to_string()), "missing {expected}");
    }
}

#[test]
fn model_declares_the_requested_columns() {
    let (tmp, _) = generate("product", Some("title:str, price:decimal, active:bool?"));
    let model = read(tmp.path(), "src/modules/product/domain/model.py");
    assert!(model.contains("__tablename__ = \"products\""));
    assert!(model.contains("title: Mapped[str] = mapped_column(String(255))"));
    assert!(model.contains("price: Mapped[Decimal] = mapped_column(Numeric(12, 2))"));
    assert!(model.contains("active: Mapped[bool | None] = mapped_column(Boolean, nullable=True)"));
    assert!(
        model.contains("from decimal import Decimal"),
        "Decimal must be imported"
    );
    assert!(
        model.contains("from sqlalchemy import Boolean, Numeric, String"),
        "column types must be imported"
    );
}

#[test]
fn schemas_separate_create_update_and_read() {
    let (tmp, _) = generate("product", Some("title:str, note:text?"));
    let schemas = read(tmp.path(), "src/modules/product/application/schemas.py");
    assert!(schemas.contains("class ProductCreate(ProductBase):"));
    assert!(schemas.contains("class ProductUpdate(BaseModel):"));
    assert!(schemas.contains("model_config = ConfigDict(from_attributes=True)"));
    assert!(schemas.contains("class ProductPage(BaseModel):"));
    assert!(
        schemas.contains("note: str | None = None"),
        "an optional field must default to None or it stays required"
    );
    assert!(
        schemas.contains("    title: str\n"),
        "a required field must not have a default"
    );
}

#[test]
fn service_maps_payloads_onto_the_entity() {
    let (tmp, _) = generate("product", Some("title:str"));
    let service = read(
        tmp.path(),
        "src/modules/product/application/product_service.py",
    );
    // The module sketch drops the payload on the floor; a resource must not.
    assert!(
        service.contains("data.model_dump(exclude_unset=True)"),
        "create must map the payload onto the entity"
    );
    assert!(service.contains("setattr(entity, field, value)"));
    assert!(
        service.contains("self._entities.count()"),
        "the page total must come from a real count"
    );
}

#[test]
fn router_exposes_paginated_crud() {
    let (tmp, _) = generate("product", Some("title:str"));
    let router = read(tmp.path(), "src/modules/product/api/router.py");
    for verb in [
        "@router.post",
        "@router.get(\"\", ",
        "@router.patch",
        "@router.delete",
    ] {
        assert!(router.contains(verb), "router is missing {verb}");
    }
    assert!(
        router.contains("Query(ge=1, le=MAX_LIMIT)"),
        "limit must be bounded"
    );
    assert!(
        router.contains("Query(ge=0)"),
        "offset must be non-negative"
    );
    assert!(router.contains("prefix=\"/products\""));
}

#[test]
fn tests_are_generated_for_the_resource() {
    let (tmp, _) = generate("product", Some("title:str, stock:int"));
    let tests = read(tmp.path(), "src/modules/product/tests/test_product.py");
    assert!(
        tests.contains("\"title\": \"widget\""),
        "payload comes from the spec"
    );
    assert!(tests.contains("\"stock\": 1"));
    assert!(
        !tests.contains("\"stock\": None"),
        "optional fields stay out of the payload"
    );
    assert!(tests.contains("test_create_read_update_delete"));
    assert!(tests.contains("assert created.status_code == 201"));
}

#[test]
fn the_test_payload_covers_optional_fields_too() {
    let (tmp, _) = generate("product", Some("title:str, note:text?, flag:bool?"));
    let tests = read(tmp.path(), "src/modules/product/tests/test_product.py");
    assert!(tests.contains("\"title\""));
    assert!(
        tests.contains("\"note\""),
        "an optional field must still be exercised end to end"
    );
    assert!(tests.contains("\"flag\""));
}

#[test]
fn a_resource_of_only_optional_fields_has_valid_tests() {
    let (tmp, _) = generate("product", Some("note:text?"));
    let tests = read(tmp.path(), "src/modules/product/tests/test_product.py");
    assert!(tests.contains("\"note\""), "the payload must not be empty");
    assert!(
        !tests.contains("missing required field"),
        "with no required field, an empty payload is valid"
    );
}

#[test]
fn dry_run_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let files =
        generate_resource("product", tmp.path(), None, Some("title:str"), false, true).unwrap();
    assert!(!files.is_empty());
    assert!(
        files.iter().all(|f| !f.content.is_empty()),
        "dry run still renders content"
    );
    assert!(
        !tmp.path().join("src").exists(),
        "dry run must not touch the disk"
    );
}

#[test]
fn refuses_to_overwrite_without_force() {
    let tmp = project();
    generate_resource("product", tmp.path(), None, Some("title:str"), false, false).unwrap();
    let second =
        generate_resource("product", tmp.path(), None, Some("title:str"), false, false).unwrap();
    assert!(
        second
            .iter()
            .all(|f| f.status == fastgen_cli::writers::Status::Skipped),
        "an existing resource must be left alone"
    );
}

#[test]
fn naming_is_applied_to_names_and_paths() {
    let (tmp, paths) = generate("OrderItem", Some("title:str"));
    let registry_path = tmp.path().join("src/modules/order_item/domain/model.py");
    assert!(
        paths.contains(&registry_path),
        "the directory is snake_case"
    );
    let model = read(tmp.path(), "src/modules/order_item/domain/model.py");
    assert!(model.contains("class OrderItem(Base):"));
    assert!(
        model.contains("__tablename__ = \"order_items\""),
        "plural + snake_case"
    );
}
