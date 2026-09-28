//! Field-spec parsing for `fastgen make resource --fields`.
//!
//! A spec is a comma-separated list of `name:type` items, where `type` may carry
//! a `?` suffix marking the field optional (nullable column, `X | None` type,
//! omitted from the test payload):
//!
//! ```text
//! title:str, price:float, active:bool?, due:date?
//! ```

use anyhow::bail;

/// The scalar field types a resource understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Str,
    Text,
    Int,
    Float,
    Decimal,
    Bool,
    DateTime,
    Date,
    Uuid,
}

impl FieldType {
    /// Every accepted type name, in help order.
    pub const NAMES: &'static [&'static str] = &[
        "str", "text", "int", "float", "decimal", "bool", "datetime", "date", "uuid",
    ];

    fn parse(name: &str) -> Option<Self> {
        match name {
            "str" => Some(Self::Str),
            "text" => Some(Self::Text),
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "decimal" => Some(Self::Decimal),
            "bool" => Some(Self::Bool),
            "datetime" => Some(Self::DateTime),
            "date" => Some(Self::Date),
            "uuid" => Some(Self::Uuid),
            _ => None,
        }
    }

    /// The annotation used in schemas (without the optional union).
    #[must_use]
    pub fn py_type(self) -> &'static str {
        match self {
            Self::Str | Self::Text => "str",
            Self::Int => "int",
            Self::Float => "float",
            Self::Decimal => "Decimal",
            Self::Bool => "bool",
            Self::DateTime => "datetime",
            Self::Date => "date",
            Self::Uuid => "UUID",
        }
    }

    /// The bare `sqlalchemy` type name (e.g. `String`) for the import block.
    #[must_use]
    pub fn col_type(self) -> &'static str {
        match self {
            Self::Str => "String",
            Self::Text => "Text",
            Self::Int => "Integer",
            Self::Float => "Float",
            Self::Decimal => "Numeric",
            Self::Bool => "Boolean",
            Self::DateTime => "DateTime",
            Self::Date => "Date",
            Self::Uuid => "Uuid",
        }
    }

    /// The `mapped_column` argument for this type.
    #[must_use]
    pub fn column(self) -> &'static str {
        match self {
            Self::Str => "String(255)",
            Self::Text => "Text",
            Self::Int => "Integer",
            Self::Float => "Float",
            Self::Decimal => "Numeric(12, 2)",
            Self::Bool => "Boolean",
            Self::DateTime => "DateTime(timezone=True)",
            Self::Date => "Date",
            Self::Uuid => "Uuid(as_uuid=True)",
        }
    }

    /// A value that pydantic accepts for this type, used in generated tests.
    #[must_use]
    pub fn sample(self) -> &'static str {
        match self {
            Self::Str => "\"widget\"",
            Self::Text => "\"a longer piece of text\"",
            Self::Int => "1",
            Self::Float => "9.99",
            Self::Decimal => "\"12.34\"",
            Self::Bool => "True",
            Self::DateTime => "\"2024-01-01T00:00:00Z\"",
            Self::Date => "\"2024-01-01\"",
            Self::Uuid => "\"3f2504e0-4f89-41d3-9a0c-0305e82c3301\"",
        }
    }

    /// The import the ORM model needs, if any.
    #[must_use]
    pub fn py_import(self) -> Option<&'static str> {
        match self {
            Self::Decimal => Some("from decimal import Decimal"),
            Self::DateTime => Some("from datetime import datetime"),
            Self::Date => Some("from datetime import date"),
            // `uuid4` is the column default, so only the model needs it.
            Self::Uuid => Some("from uuid import UUID, uuid4"),
            _ => None,
        }
    }

    /// The import the schemas need, if any.
    #[must_use]
    pub fn py_import_schema(self) -> Option<&'static str> {
        match self {
            Self::Uuid => Some("from uuid import UUID"),
            _ => self.py_import(),
        }
    }
}

/// One declared field of a resource.
#[derive(Debug, Clone)]
pub struct Field {
    /// The attribute name (already `snake_case`).
    pub name: String,
    /// The declared scalar type.
    pub ty: FieldType,
    /// Whether the field is optional (`type?`).
    pub optional: bool,
}

impl Field {
    /// The full annotation, e.g. `str` or `int | None`.
    #[must_use]
    pub fn annotation(&self) -> String {
        if self.optional {
            format!("{} | None", self.ty.py_type())
        } else {
            self.ty.py_type().to_string()
        }
    }

    /// The annotation used in the PATCH schema, where every field is optional
    /// regardless of how it was declared.
    #[must_use]
    pub fn update_annotation(&self) -> String {
        format!("{} | None", self.ty.py_type())
    }

    /// The full `mapped_column(...)` expression.
    #[must_use]
    pub fn column_expr(&self) -> String {
        let mut args = vec![self.ty.column().to_string()];
        if self.optional {
            args.push("nullable=True".to_string());
        } else if self.ty == FieldType::Uuid {
            args.push("default=uuid4".to_string());
        }
        format!("mapped_column({})", args.join(", "))
    }
}

/// The field set of a resource: the parsed `--fields` spec.
#[derive(Debug, Clone)]
pub struct Fields {
    pub items: Vec<Field>,
}

impl Fields {
    /// The field set used when `--fields` is omitted: a single `name: str`.
    #[must_use]
    pub fn fallback() -> Self {
        Self {
            items: vec![Field {
                name: "name".to_string(),
                ty: FieldType::Str,
                optional: false,
            }],
        }
    }

    /// The distinct `sqlalchemy` type names the field set requires, sorted.
    #[must_use]
    pub fn column_types(&self) -> Vec<&'static str> {
        let mut types: Vec<&'static str> = self.items.iter().map(|f| f.ty.col_type()).collect();
        types.sort_unstable();
        types.dedup();
        types
    }

    /// The distinct `from ... import ...` lines the field set requires, sorted.
    #[must_use]
    pub fn imports(&self) -> Vec<String> {
        self.collect_imports(FieldType::py_import)
    }

    /// The same, for the pydantic schemas.
    #[must_use]
    pub fn schema_imports(&self) -> Vec<String> {
        self.collect_imports(FieldType::py_import_schema)
    }

    fn collect_imports(&self, pick: fn(FieldType) -> Option<&'static str>) -> Vec<String> {
        let mut imports: Vec<String> = self
            .items
            .iter()
            .filter_map(|f| pick(f.ty).map(ToString::to_string))
            .collect();
        imports.sort();
        imports.dedup();
        imports
    }

    /// The first non-optional field, used to build the generated test payload.
    #[must_use]
    pub fn first_required(&self) -> Option<&Field> {
        self.items.iter().find(|f| !f.optional)
    }
}

/// Parse a `--fields` spec such as `title:str, price:float, active:bool?`.
///
/// # Errors
///
/// Returns an error for a malformed item, an unknown type, or a duplicate name.
pub fn parse_fields(spec: &str) -> anyhow::Result<Fields> {
    let mut items: Vec<Field> = Vec::new();
    for raw in spec.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        let Some((name, ty)) = item.split_once(':') else {
            bail!(
                "invalid field '{item}': expected `name:type` (types: {})",
                FieldType::NAMES.join(", ")
            );
        };
        let name = name.trim();
        if name.is_empty() {
            bail!("invalid field '{item}': the name is empty");
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            || name.starts_with(|c: char| c.is_ascii_digit())
        {
            bail!("invalid field '{item}': '{name}' is not a valid Python identifier");
        }
        let (ty_name, optional) = match ty.trim().strip_suffix('?') {
            Some(base) => (base.trim(), true),
            None => (ty.trim(), false),
        };
        // Type names are case-insensitive: `--fields "name:Str"` is fine.
        let Some(field_ty) = FieldType::parse(&ty_name.to_ascii_lowercase()) else {
            bail!(
                "unknown type '{ty_name}' in field '{item}' (types: {})",
                FieldType::NAMES.join(", ")
            );
        };
        let name = crate::naming::to_snake(name);
        if items.iter().any(|f| f.name == name) {
            bail!("duplicate field '{name}'");
        }
        items.push(Field {
            name,
            ty: field_ty,
            optional,
        });
    }
    if items.is_empty() {
        bail!("--fields was empty: use e.g. --fields \"title:str, price:float\"");
    }
    Ok(Fields { items })
}
