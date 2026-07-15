use super::{DefinitionKind, MAX_SCHEMA_BYTES, Parameter, Schema, SchemaParseErrorKind};

const SMALL_SCHEMA: &str = r#"
//@class Widget @description A test aggregate
//@description A concrete widget
widget tags:vector<string> = Widget;

vector {t:Type} # [ t ] = Vector t;

---functions---

//@description Returns a widget
getWidget id:int53 = Widget;
"#;

#[test]
fn parses_domain_model_and_preserves_relevant_source_context() {
    let schema = Schema::parse(SMALL_SCHEMA).expect("valid schema");

    assert_eq!(schema.definitions().len(), 2);
    assert_eq!(schema.methods().len(), 1);

    let widget = &schema.definitions()[0];
    assert_eq!(widget.kind(), DefinitionKind::Constructor);
    assert_eq!(widget.name(), "widget");
    assert_eq!(widget.line(), 4);
    assert_eq!(
        widget.documentation().raw_lines(),
        [
            "@class Widget @description A test aggregate",
            "@description A concrete widget",
        ]
    );
    assert_eq!(widget.result().name(), "Widget");
    assert!(widget.result().arguments().is_empty());

    let Parameter::Field { name, ty } = &widget.parameters()[0] else {
        panic!("widget must have a named field");
    };
    assert_eq!(name, "tags");
    assert_eq!(ty.name(), "vector");
    assert_eq!(ty.arguments()[0].name(), "string");

    let vector = &schema.definitions()[1];
    assert_eq!(vector.kind(), DefinitionKind::Builtin);
    assert!(matches!(
        &vector.parameters()[0],
        Parameter::TypeParameter { name, bound }
            if name == "t" && bound.name() == "Type"
    ));
    assert!(matches!(&vector.parameters()[1], Parameter::TypeIdentifier));
    assert!(matches!(
        &vector.parameters()[2],
        Parameter::Repeated { ty } if ty.name() == "t"
    ));
    assert_eq!(vector.result().name(), "Vector");
    assert_eq!(vector.result().arguments()[0].name(), "t");

    let method = &schema.methods()[0];
    assert_eq!(method.kind(), DefinitionKind::Method);
    assert_eq!(method.name(), "getWidget");
    assert_eq!(
        method.documentation().raw_lines(),
        ["@description Returns a widget"]
    );
    assert_eq!(method.documentation().tags()[0].name(), "description");
    assert_eq!(method.documentation().tags()[0].value(), "Returns a widget");
    assert_eq!(method.canonical_signature(), "getWidget id:int53 = Widget;");
}

#[test]
fn supports_multiline_declarations_without_losing_start_line() {
    let schema =
        Schema::parse("thing\n  value:string\n  = Thing;\n---functions---\ngetThing = Thing;\n")
            .expect("multiline declaration");

    assert_eq!(schema.definitions()[0].line(), 1);
    assert_eq!(
        schema.definitions()[0].canonical_signature(),
        "thing value:string = Thing;"
    );
}

#[test]
fn builds_complete_deterministic_inventory_for_pinned_tdlib() {
    let source = include_str!("../../../../vendor/tdlib/td_api.tl");
    let schema = Schema::parse(source).expect("pinned TDLib schema must parse");
    let inventory = schema.inventory();

    assert_eq!(inventory.definition_names().len(), 2_168);
    assert_eq!(inventory.builtin_names().len(), 9);
    assert_eq!(inventory.constructor_names().len(), 2_159);
    assert_eq!(inventory.method_names().len(), 1_010);
    assert_eq!(inventory.type_names().len(), 745);
    assert_eq!(inventory.update_names().len(), 184);
    assert_eq!(inventory.authorization_state_names().len(), 13);

    assert!(
        inventory
            .method_names()
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    assert!(
        inventory
            .constructor_names()
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    assert!(inventory.method_names().binary_search(&"getMe").is_ok());
    assert!(
        inventory
            .update_names()
            .binary_search(&"updateAuthorizationState")
            .is_ok()
    );
    assert!(
        inventory
            .authorization_state_names()
            .binary_search(&"authorizationStateReady")
            .is_ok()
    );
    assert!(
        inventory
            .update_names()
            .binary_search(&"testUseUpdate")
            .is_err()
    );
    assert!(
        inventory
            .authorization_state_names()
            .binary_search(&"getAuthorizationState")
            .is_err()
    );
}

#[test]
fn parses_nested_vectors_and_structured_documentation() {
    let schema = Schema::parse(
        "//@class Box @description Type docs\n\
         //@description First line @items Nested items\n\
         //-continued without a new tag\n\
         box items:vector<vector<string>> = Box;\n\
         ---functions---\n\
         getBox = Box;\n",
    )
    .expect("nested vectors and documentation");

    let definition = &schema.definitions()[0];
    let Parameter::Field { ty, .. } = &definition.parameters()[0] else {
        panic!("items is a field");
    };
    assert_eq!(ty.name(), "vector");
    assert_eq!(ty.arguments()[0].name(), "vector");
    assert_eq!(ty.arguments()[0].arguments()[0].name(), "string");

    let tags = definition.documentation().tags();
    assert_eq!(
        tags.iter().map(|tag| tag.name()).collect::<Vec<_>>(),
        ["class", "description", "description", "items"]
    );
    assert_eq!(tags[0].value(), "Box");
    assert_eq!(tags[1].value(), "Type docs");
    assert_eq!(tags[3].value(), "Nested items\ncontinued without a new tag");
}

#[test]
fn rejects_structural_ambiguity_fail_closed() {
    let cases = [
        (
            "thing = Thing;\n",
            SchemaParseErrorKind::MissingFunctionsDelimiter,
            2,
        ),
        (
            "thing = Thing;\n---functions---\n---functions---\ngetThing = Thing;\n",
            SchemaParseErrorKind::DuplicateFunctionsDelimiter,
            3,
        ),
        (
            "thing = Thing; trailing\n---functions---\ngetThing = Thing;\n",
            SchemaParseErrorKind::TrailingCharacters,
            1,
        ),
        (
            "thing invalid = Thing;\n---functions---\ngetThing = Thing;\n",
            SchemaParseErrorKind::InvalidParameter,
            1,
        ),
        (
            "thing value:vector<string = Thing;\n---functions---\ngetThing = Thing;\n",
            SchemaParseErrorKind::InvalidType,
            1,
        ),
        (
            "thing = Thing\n---functions---\ngetThing = Thing;\n",
            SchemaParseErrorKind::UnterminatedDefinition,
            1,
        ),
        (
            "thing = Thing;\n---functions---\nthing = Thing;\n",
            SchemaParseErrorKind::DuplicateName,
            3,
        ),
    ];

    for (source, expected_kind, expected_line) in cases {
        let error = Schema::parse(source).expect_err("malformed schema must fail");
        assert_eq!(error.kind(), expected_kind, "source: {source:?}");
        assert_eq!(error.line(), expected_line, "source: {source:?}");
    }
}

#[test]
fn rejects_syntax_outside_the_pinned_tdlib_subset() {
    let cases = [
        "thing#01234567 = Thing;\n---functions---\ngetThing = Thing;\n",
        "thing flags:# value:flags.0?string = Thing;\n---functions---\ngetThing = Thing;\n",
        "ns.thing = Thing;\n---functions---\ngetThing = Thing;\n",
        "thing value:!X = Thing;\n---functions---\ngetThing = Thing;\n",
        "thing values:list<string> = Thing;\n---functions---\ngetThing = Thing;\n",
        "thing ? = Thing;\n---functions---\ngetThing = Thing;\n",
        "thing = Thing Extra;\n---functions---\ngetThing = Thing;\n",
    ];

    for source in cases {
        let error = Schema::parse(source).expect_err("unsupported syntax must fail");
        assert_eq!(
            error.kind(),
            SchemaParseErrorKind::UnsupportedSyntax,
            "source: {source:?}"
        );
        assert_eq!(error.line(), 1);
    }
}

#[test]
fn rejects_duplicate_fields() {
    let error = Schema::parse(
        "thing value:string value:int32 = Thing;\n---functions---\ngetThing = Thing;\n",
    )
    .expect_err("duplicate field");

    assert_eq!(error.kind(), SchemaParseErrorKind::InvalidParameter);
}

#[test]
fn rejects_excessive_type_nesting_without_recursing_unboundedly() {
    let nested = format!("{}string{}", "vector<".repeat(40), ">".repeat(40));
    let source = format!("thing value:{nested} = Thing;\n---functions---\ngetThing = Thing;\n");
    let error = Schema::parse(&source).expect_err("nesting must be bounded");

    assert_eq!(error.kind(), SchemaParseErrorKind::TypeNestingLimit);
    assert_eq!(error.line(), 1);
}

#[test]
fn rejects_unresolved_type_references() {
    let error =
        Schema::parse("thing value:MissingType = Thing;\n---functions---\ngetThing = Thing;\n")
            .expect_err("unresolved field type");

    assert_eq!(error.kind(), SchemaParseErrorKind::UnresolvedType);
    assert_eq!(error.line(), 1);
}

#[test]
fn rejects_identifiers_outside_their_tl_lexical_roles() {
    let cases = [
        "Thing = Thing;\n---functions---\ngetThing = Thing;\n",
        "thing = Thing;\n---functions---\nGetThing = Thing;\n",
        "_thing _value:string = _Thing;\n---functions---\ngetThing = _Thing;\n",
        "thing = thing;\n---functions---\ngetThing = thing;\n",
        "thing = Final;\n---functions---\ngetThing = Final;\n",
    ];

    for source in cases {
        let error = Schema::parse(source).expect_err("invalid lexical role must fail");
        assert_eq!(
            error.kind(),
            SchemaParseErrorKind::UnsupportedSyntax,
            "source: {source:?}"
        );
    }
}

#[test]
fn rejects_unapplied_or_ill_kinded_vector_types() {
    let declarations = [
        "thing items:vector = Thing;",
        "thing items:vector<vector> = Thing;",
        "thing items:Vector = Thing;",
    ];
    let builtins = "string ? = String;\nvector {t:Type} # [ t ] = Vector t;\n";

    for declaration in declarations {
        let source = format!("{builtins}{declaration}\n---functions---\ngetThing = Thing;\n");
        let error = Schema::parse(&source).expect_err("invalid vector arity must fail");
        assert_eq!(
            error.kind(),
            SchemaParseErrorKind::UnsupportedSyntax,
            "declaration: {declaration}"
        );
    }

    let source = format!("{builtins}thing = Thing;\n---functions---\ngetThings = Vector;\n");
    let error = Schema::parse(&source).expect_err("unapplied Vector result must fail");
    assert_eq!(error.kind(), SchemaParseErrorKind::UnsupportedSyntax);
}

#[test]
fn rejects_source_over_the_canonical_schema_byte_cap_before_parsing() {
    let oversized = "x".repeat(MAX_SCHEMA_BYTES + 1);
    let error = Schema::parse(&oversized).expect_err("oversized schema must fail early");

    assert_eq!(error.kind(), SchemaParseErrorKind::SourceTooLarge);
    assert_eq!(error.line(), 1);
}
