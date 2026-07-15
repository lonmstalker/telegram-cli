//! Строгая доменная модель закреплённого `td_api.tl`.
//!
//! TDLib использует небольшой subset TL: до delimiter объявлены constructors,
//! после него — methods. Parser намеренно не угадывает неизвестный синтаксис:
//! schema drift должен остановить generation раньше runtime.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const FUNCTIONS_DELIMITER: &str = "---functions---";
const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
const MAX_TYPE_NESTING: usize = 32;
const BUILTIN_NAMES: [&str; 9] = [
    "double",
    "string",
    "int32",
    "int53",
    "int64",
    "bytes",
    "boolFalse",
    "boolTrue",
    "vector",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionKind {
    Builtin,
    Constructor,
    Method,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeRef {
    name: String,
    arguments: Vec<TypeRef>,
}

impl TypeRef {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arguments(&self) -> &[TypeRef] {
        &self.arguments
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Parameter {
    Field { name: String, ty: TypeRef },
    TypeParameter { name: String, bound: TypeRef },
    TypeIdentifier,
    Repeated { ty: TypeRef },
    Anonymous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentationTag {
    name: String,
    value: String,
}

impl DocumentationTag {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Documentation {
    raw_lines: Vec<String>,
    tags: Vec<DocumentationTag>,
}

impl Documentation {
    fn parse(raw_lines: Vec<String>) -> Self {
        let mut tags: Vec<DocumentationTag> = Vec::new();
        for line in &raw_lines {
            if let Some(continuation) = line.strip_prefix('-') {
                if let Some(tag) = tags.last_mut() {
                    if !tag.value.is_empty() {
                        tag.value.push('\n');
                    }
                    tag.value.push_str(continuation.trim());
                }
                continue;
            }
            tags.extend(parse_documentation_tags(line));
        }
        Self { raw_lines, tags }
    }

    pub fn raw_lines(&self) -> &[String] {
        &self.raw_lines
    }

    pub fn tags(&self) -> &[DocumentationTag] {
        &self.tags
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    kind: DefinitionKind,
    name: String,
    parameters: Vec<Parameter>,
    result: TypeRef,
    documentation: Documentation,
    line: usize,
    canonical_signature: String,
}

impl Definition {
    pub fn kind(&self) -> DefinitionKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn result(&self) -> &TypeRef {
        &self.result
    }

    pub fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn canonical_signature(&self) -> &str {
        &self.canonical_signature
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    definitions: Vec<Definition>,
    methods: Vec<Definition>,
}

impl Schema {
    pub fn parse(source: &str) -> Result<Self, SchemaParseError> {
        if source.len() > MAX_SCHEMA_BYTES {
            return Err(SchemaParseError::new(
                SchemaParseErrorKind::SourceTooLarge,
                1,
                format!(
                    "schema exceeds the {MAX_SCHEMA_BYTES}-byte input cap: {}",
                    source.len()
                ),
            ));
        }
        Parser::new(source).parse()
    }

    pub fn definitions(&self) -> &[Definition] {
        &self.definitions
    }

    pub fn methods(&self) -> &[Definition] {
        &self.methods
    }

    pub fn inventory(&self) -> SchemaInventory<'_> {
        SchemaInventory::from_schema(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaInventory<'schema> {
    definition_names: Vec<&'schema str>,
    builtin_names: Vec<&'schema str>,
    constructor_names: Vec<&'schema str>,
    method_names: Vec<&'schema str>,
    type_names: Vec<&'schema str>,
    update_names: Vec<&'schema str>,
    authorization_state_names: Vec<&'schema str>,
}

impl<'schema> SchemaInventory<'schema> {
    fn from_schema(schema: &'schema Schema) -> Self {
        let mut definition_names = sorted_names(&schema.definitions);
        let mut builtin_names = names_with_kind(&schema.definitions, DefinitionKind::Builtin);
        let mut constructor_names =
            names_with_kind(&schema.definitions, DefinitionKind::Constructor);
        let mut method_names = sorted_names(&schema.methods);
        let type_names = schema
            .definitions
            .iter()
            .map(|definition| definition.result.name())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut update_names = names_with_result(&schema.definitions, "Update");
        let mut authorization_state_names =
            names_with_result(&schema.definitions, "AuthorizationState");

        definition_names.sort_unstable();
        builtin_names.sort_unstable();
        constructor_names.sort_unstable();
        method_names.sort_unstable();
        update_names.sort_unstable();
        authorization_state_names.sort_unstable();

        Self {
            definition_names,
            builtin_names,
            constructor_names,
            method_names,
            type_names,
            update_names,
            authorization_state_names,
        }
    }

    pub fn definition_names(&self) -> &[&'schema str] {
        &self.definition_names
    }

    pub fn builtin_names(&self) -> &[&'schema str] {
        &self.builtin_names
    }

    pub fn constructor_names(&self) -> &[&'schema str] {
        &self.constructor_names
    }

    pub fn method_names(&self) -> &[&'schema str] {
        &self.method_names
    }

    pub fn type_names(&self) -> &[&'schema str] {
        &self.type_names
    }

    pub fn update_names(&self) -> &[&'schema str] {
        &self.update_names
    }

    pub fn authorization_state_names(&self) -> &[&'schema str] {
        &self.authorization_state_names
    }
}

fn sorted_names(definitions: &[Definition]) -> Vec<&str> {
    definitions
        .iter()
        .map(|definition| definition.name())
        .collect()
}

fn names_with_result<'schema>(
    definitions: &'schema [Definition],
    result_name: &str,
) -> Vec<&'schema str> {
    definitions
        .iter()
        .filter(|definition| definition.result.name() == result_name)
        .map(|definition| definition.name())
        .collect()
}

fn names_with_kind(definitions: &[Definition], kind: DefinitionKind) -> Vec<&str> {
    definitions
        .iter()
        .filter(|definition| definition.kind == kind)
        .map(|definition| definition.name())
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaParseErrorKind {
    MissingFunctionsDelimiter,
    DuplicateFunctionsDelimiter,
    UnterminatedDefinition,
    TrailingCharacters,
    InvalidDefinition,
    InvalidParameter,
    InvalidType,
    DuplicateName,
    UnsupportedSyntax,
    TypeNestingLimit,
    UnresolvedType,
    SourceTooLarge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaParseError {
    kind: SchemaParseErrorKind,
    line: usize,
    detail: String,
}

impl SchemaParseError {
    fn new(kind: SchemaParseErrorKind, line: usize, detail: impl Into<String>) -> Self {
        Self {
            kind,
            line,
            detail: detail.into(),
        }
    }

    pub fn kind(&self) -> SchemaParseErrorKind {
        self.kind
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SchemaParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TDLib schema parse error at line {}: {}",
            self.line, self.detail
        )
    }
}

impl Error for SchemaParseError {}

fn parse_documentation_tags(line: &str) -> Vec<DocumentationTag> {
    let bytes = line.as_bytes();
    let mut markers = Vec::new();
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position] != b'@' || (position > 0 && !bytes[position - 1].is_ascii_whitespace()) {
            position += 1;
            continue;
        }

        let name_start = position + 1;
        let mut name_end = name_start;
        while bytes
            .get(name_end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            name_end += 1;
        }
        if name_end == name_start
            || bytes
                .get(name_end)
                .is_some_and(|byte| !byte.is_ascii_whitespace())
        {
            position += 1;
            continue;
        }
        markers.push((position, name_start, name_end));
        position = name_end;
    }

    markers
        .iter()
        .enumerate()
        .map(|(index, (_, name_start, name_end))| {
            let value_end = markers
                .get(index + 1)
                .map_or(line.len(), |(marker, _, _)| *marker);
            DocumentationTag {
                name: line[*name_start..*name_end].to_owned(),
                value: line[*name_end..value_end].trim().to_owned(),
            }
        })
        .collect()
}

struct Parser<'source> {
    source: &'source str,
    kind: DefinitionKind,
    delimiter_seen: bool,
    definitions: Vec<Definition>,
    methods: Vec<Definition>,
    names: BTreeSet<String>,
    documentation: Vec<String>,
    statement: String,
    statement_line: Option<usize>,
}

impl<'source> Parser<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            kind: DefinitionKind::Constructor,
            delimiter_seen: false,
            definitions: Vec::new(),
            methods: Vec::new(),
            names: BTreeSet::new(),
            documentation: Vec::new(),
            statement: String::new(),
            statement_line: None,
        }
    }

    fn parse(mut self) -> Result<Schema, SchemaParseError> {
        for (index, raw_line) in self.source.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();

            if line.is_empty() {
                continue;
            }
            if let Some(comment) = line.strip_prefix("//") {
                if self.statement_line.is_some() {
                    return Err(self.error(
                        SchemaParseErrorKind::InvalidDefinition,
                        line_number,
                        "comment inside a declaration",
                    ));
                }
                self.documentation.push(comment.trim().to_owned());
                continue;
            }
            if line == FUNCTIONS_DELIMITER {
                self.enter_function_section(line_number)?;
                continue;
            }

            self.push_declaration_line(line, line_number)?;
        }

        if let Some(line) = self.statement_line {
            return Err(self.error(
                SchemaParseErrorKind::UnterminatedDefinition,
                line,
                "declaration has no terminating semicolon",
            ));
        }
        if !self.delimiter_seen {
            return Err(self.error(
                SchemaParseErrorKind::MissingFunctionsDelimiter,
                self.source.lines().count() + 1,
                "missing ---functions--- delimiter",
            ));
        }

        let schema = Schema {
            definitions: self.definitions,
            methods: self.methods,
        };
        validate_type_references(&schema)?;
        Ok(schema)
    }

    fn enter_function_section(&mut self, line: usize) -> Result<(), SchemaParseError> {
        if let Some(start_line) = self.statement_line {
            return Err(self.error(
                SchemaParseErrorKind::UnterminatedDefinition,
                start_line,
                "declaration reaches ---functions--- without a semicolon",
            ));
        }
        if self.delimiter_seen {
            return Err(self.error(
                SchemaParseErrorKind::DuplicateFunctionsDelimiter,
                line,
                "duplicate ---functions--- delimiter",
            ));
        }
        self.delimiter_seen = true;
        self.kind = DefinitionKind::Method;
        self.documentation.clear();
        Ok(())
    }

    fn push_declaration_line(
        &mut self,
        line: &str,
        line_number: usize,
    ) -> Result<(), SchemaParseError> {
        let (fragment, terminated) = match line.split_once(';') {
            Some((fragment, trailing)) => {
                if !trailing.trim().is_empty() {
                    return Err(self.error(
                        SchemaParseErrorKind::TrailingCharacters,
                        line_number,
                        "characters after declaration terminator",
                    ));
                }
                (fragment.trim(), true)
            }
            None => (line, false),
        };

        if self.statement_line.is_none() {
            self.statement_line = Some(line_number);
        }
        if !fragment.is_empty() {
            if !self.statement.is_empty() {
                self.statement.push(' ');
            }
            self.statement.push_str(fragment);
        }
        if !terminated {
            return Ok(());
        }

        let start_line = self.statement_line.take().expect("line was assigned");
        let signature = format!("{};", self.statement.trim());
        self.statement.clear();
        let documentation = std::mem::take(&mut self.documentation);
        let definition = parse_definition(self.kind, signature, documentation, start_line)?;
        if !self.names.insert(definition.name.clone()) {
            return Err(self.error(
                SchemaParseErrorKind::DuplicateName,
                start_line,
                format!("duplicate definition name `{}`", definition.name),
            ));
        }
        match self.kind {
            DefinitionKind::Builtin | DefinitionKind::Constructor => {
                self.definitions.push(definition);
            }
            DefinitionKind::Method => self.methods.push(definition),
        }
        Ok(())
    }

    fn error(
        &self,
        kind: SchemaParseErrorKind,
        line: usize,
        detail: impl Into<String>,
    ) -> SchemaParseError {
        SchemaParseError::new(kind, line, detail)
    }
}

fn parse_definition(
    kind: DefinitionKind,
    signature: String,
    documentation: Vec<String>,
    line: usize,
) -> Result<Definition, SchemaParseError> {
    let body = signature
        .strip_suffix(';')
        .expect("parser always supplies a terminated signature")
        .trim();
    if body.contains("/*")
        || body.contains("*/")
        || body.contains('!')
        || body.contains('%')
        || body.contains('(')
        || body.contains(')')
        || body.starts_with("---")
    {
        return Err(parse_error(
            SchemaParseErrorKind::UnsupportedSyntax,
            line,
            "declaration uses syntax outside the pinned TDLib subset",
        ));
    }
    let mut equals = body.match_indices('=');
    let Some((equals_index, _)) = equals.next() else {
        return Err(parse_error(
            SchemaParseErrorKind::InvalidDefinition,
            line,
            "declaration has no result separator",
        ));
    };
    if equals.next().is_some() {
        return Err(parse_error(
            SchemaParseErrorKind::InvalidDefinition,
            line,
            "declaration has multiple result separators",
        ));
    }

    let left = body[..equals_index].trim();
    let result_source = body[equals_index + 1..].trim();
    if left.is_empty() || result_source.is_empty() {
        return Err(parse_error(
            SchemaParseErrorKind::InvalidDefinition,
            line,
            "declaration name and result type are required",
        ));
    }

    let head_end = left.find(char::is_whitespace).unwrap_or(left.len());
    let head = &left[..head_end];
    let parameters_source = left[head_end..].trim();
    let name = parse_definition_head(head, line)?;
    let parameters = parse_parameters(parameters_source, line)?;
    let result = parse_type(result_source, true, line)?;
    if !is_boxed_result_identifier(result.name()) {
        return Err(unsupported_syntax(
            line,
            format!(
                "result type `{}` must be a non-reserved boxed type",
                result.name()
            ),
        ));
    }
    let kind = if kind == DefinitionKind::Constructor && BUILTIN_NAMES.contains(&name.as_str()) {
        DefinitionKind::Builtin
    } else {
        kind
    };
    validate_pinned_definition(kind, &name, &parameters, &result, line)?;
    let canonical_signature = canonical_signature(&name, &parameters, &result);

    Ok(Definition {
        kind,
        name,
        parameters,
        result,
        documentation: Documentation::parse(documentation),
        line,
        canonical_signature,
    })
}

fn canonical_signature(name: &str, parameters: &[Parameter], result: &TypeRef) -> String {
    let mut signature = name.to_owned();
    for parameter in parameters {
        signature.push(' ');
        match parameter {
            Parameter::Field { name, ty } => {
                signature.push_str(name);
                signature.push(':');
                signature.push_str(&canonical_type(ty));
            }
            Parameter::TypeParameter { name, bound } => {
                signature.push('{');
                signature.push_str(name);
                signature.push(':');
                signature.push_str(&canonical_type(bound));
                signature.push('}');
            }
            Parameter::TypeIdentifier => signature.push('#'),
            Parameter::Repeated { ty } => {
                signature.push_str("[ ");
                signature.push_str(&canonical_type(ty));
                signature.push_str(" ]");
            }
            Parameter::Anonymous => signature.push('?'),
        }
    }
    signature.push_str(" = ");
    signature.push_str(&canonical_result_type(result));
    signature.push(';');
    signature
}

fn canonical_type(ty: &TypeRef) -> String {
    if ty.arguments.is_empty() {
        return ty.name.clone();
    }
    let arguments = ty
        .arguments
        .iter()
        .map(canonical_type)
        .collect::<Vec<_>>()
        .join(",");
    format!("{}<{arguments}>", ty.name)
}

fn canonical_result_type(ty: &TypeRef) -> String {
    if ty.arguments.is_empty() {
        return ty.name.clone();
    }
    let arguments = ty
        .arguments
        .iter()
        .map(canonical_type)
        .collect::<Vec<_>>()
        .join(" ");
    format!("{} {arguments}", ty.name)
}

fn validate_pinned_definition(
    kind: DefinitionKind,
    name: &str,
    parameters: &[Parameter],
    result: &TypeRef,
    line: usize,
) -> Result<(), SchemaParseError> {
    if kind == DefinitionKind::Builtin {
        if valid_builtin_shape(name, parameters, result) {
            return Ok(());
        }
        return Err(unsupported_syntax(
            line,
            format!("builtin `{name}` does not match the pinned TDLib shape"),
        ));
    }

    if parameters
        .iter()
        .any(|parameter| !matches!(parameter, Parameter::Field { .. }))
    {
        return Err(unsupported_syntax(
            line,
            "anonymous and generic parameters are only valid in pinned builtins",
        ));
    }
    for parameter in parameters {
        if let Parameter::Field { ty, .. } = parameter {
            validate_field_type(ty, line)?;
        }
    }
    if result.name == "Vector" || !result.arguments.is_empty() {
        return Err(unsupported_syntax(
            line,
            "Vector result application is only valid in the pinned vector builtin",
        ));
    }
    Ok(())
}

fn valid_builtin_shape(name: &str, parameters: &[Parameter], result: &TypeRef) -> bool {
    let expected_scalar_result = match name {
        "double" => Some(("Double", true)),
        "string" => Some(("String", true)),
        "int32" => Some(("Int32", false)),
        "int53" => Some(("Int53", false)),
        "int64" => Some(("Int64", false)),
        "bytes" => Some(("Bytes", false)),
        "boolFalse" | "boolTrue" => Some(("Bool", false)),
        _ => None,
    };
    if let Some((expected_result, anonymous)) = expected_scalar_result {
        let parameters_match = if anonymous {
            matches!(parameters, [Parameter::Anonymous])
        } else {
            parameters.is_empty()
        };
        return parameters_match && result.name == expected_result && result.arguments.is_empty();
    }

    name == "vector"
        && matches!(
            parameters,
            [
                Parameter::TypeParameter { name, bound },
                Parameter::TypeIdentifier,
                Parameter::Repeated { ty },
            ] if name == "t"
                && bound.name == "Type"
                && bound.arguments.is_empty()
                && ty.name == "t"
                && ty.arguments.is_empty()
        )
        && result.name == "Vector"
        && matches!(result.arguments.as_slice(), [argument] if argument.name == "t" && argument.arguments.is_empty())
}

fn validate_field_type(ty: &TypeRef, line: usize) -> Result<(), SchemaParseError> {
    if is_reserved_type_name(ty.name()) || ty.name == "Vector" {
        return Err(unsupported_syntax(
            line,
            format!("type `{}` is not a concrete field type", ty.name),
        ));
    }
    if ty.name == "vector" {
        if ty.arguments.len() != 1 {
            return Err(unsupported_syntax(
                line,
                "vector field type requires exactly one type argument",
            ));
        }
        return validate_field_type(&ty.arguments[0], line);
    }
    if !ty.arguments.is_empty() {
        return Err(unsupported_syntax(
            line,
            format!(
                "generic type `{}` is outside the pinned TDLib subset",
                ty.name
            ),
        ));
    }
    Ok(())
}

fn unsupported_syntax(line: usize, detail: impl Into<String>) -> SchemaParseError {
    parse_error(SchemaParseErrorKind::UnsupportedSyntax, line, detail)
}

fn validate_type_references(schema: &Schema) -> Result<(), SchemaParseError> {
    let mut known = BUILTIN_NAMES.into_iter().collect::<BTreeSet<_>>();
    known.extend([
        "Double", "String", "Int32", "Int53", "Int64", "Bytes", "Bool", "Vector", "Type",
    ]);
    for definition in &schema.definitions {
        known.insert(definition.name());
        known.insert(definition.result.name());
    }

    for definition in schema.definitions.iter().chain(&schema.methods) {
        let type_parameters = definition
            .parameters
            .iter()
            .filter_map(|parameter| match parameter {
                Parameter::TypeParameter { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        for parameter in &definition.parameters {
            let ty = match parameter {
                Parameter::Field { ty, .. }
                | Parameter::TypeParameter { bound: ty, .. }
                | Parameter::Repeated { ty } => Some(ty),
                Parameter::TypeIdentifier | Parameter::Anonymous => None,
            };
            if let Some(ty) = ty {
                validate_type_reference(ty, &known, &type_parameters, definition.line)?;
            }
        }
        validate_type_reference(
            &definition.result,
            &known,
            &type_parameters,
            definition.line,
        )?;
    }
    Ok(())
}

fn validate_type_reference(
    ty: &TypeRef,
    known: &BTreeSet<&str>,
    type_parameters: &BTreeSet<&str>,
    line: usize,
) -> Result<(), SchemaParseError> {
    if !known.contains(ty.name()) && !type_parameters.contains(ty.name()) {
        return Err(parse_error(
            SchemaParseErrorKind::UnresolvedType,
            line,
            format!("unresolved type `{}`", ty.name),
        ));
    }
    for argument in &ty.arguments {
        validate_type_reference(argument, known, type_parameters, line)?;
    }
    Ok(())
}

fn parse_definition_head(head: &str, line: usize) -> Result<String, SchemaParseError> {
    if head.contains('#') || head.contains('.') {
        return Err(parse_error(
            SchemaParseErrorKind::UnsupportedSyntax,
            line,
            "explicit constructor ids and namespaces are not in the pinned TDLib subset",
        ));
    }
    if !is_lowercase_identifier(head) {
        return Err(parse_error(
            SchemaParseErrorKind::UnsupportedSyntax,
            line,
            format!("combinator name `{head}` must start with a lowercase letter"),
        ));
    }
    Ok(head.to_owned())
}

fn parse_parameters(source: &str, line: usize) -> Result<Vec<Parameter>, SchemaParseError> {
    let mut parameters = Vec::new();
    let mut named_parameters = BTreeSet::new();
    let mut cursor = ParameterCursor::new(source);
    while cursor.skip_whitespace() {
        let parameter = cursor.parse_parameter(line)?;
        let name = match &parameter {
            Parameter::Field { name, .. } | Parameter::TypeParameter { name, .. } => Some(name),
            Parameter::TypeIdentifier | Parameter::Repeated { .. } | Parameter::Anonymous => None,
        };
        if let Some(name) = name
            && !named_parameters.insert(name.clone())
        {
            return Err(parse_error(
                SchemaParseErrorKind::InvalidParameter,
                line,
                format!("duplicate parameter `{name}`"),
            ));
        }
        parameters.push(parameter);
    }
    Ok(parameters)
}

struct ParameterCursor<'source> {
    source: &'source str,
    position: usize,
}

impl<'source> ParameterCursor<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn skip_whitespace(&mut self) -> bool {
        while self
            .source
            .as_bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
        self.position < self.source.len()
    }

    fn parse_parameter(&mut self, line: usize) -> Result<Parameter, SchemaParseError> {
        match self.source.as_bytes()[self.position] {
            b'?' => {
                self.position += 1;
                self.require_boundary(line)?;
                Ok(Parameter::Anonymous)
            }
            b'#' => {
                self.position += 1;
                self.require_boundary(line)?;
                Ok(Parameter::TypeIdentifier)
            }
            b'{' => {
                let body = self.take_delimited(b'{', b'}', line)?;
                let (name, bound) = parse_named_type(body, line)?;
                Ok(Parameter::TypeParameter { name, bound })
            }
            b'[' => {
                let body = self.take_delimited(b'[', b']', line)?;
                let ty = parse_type(body.trim(), false, line)?;
                Ok(Parameter::Repeated { ty })
            }
            _ => {
                let start = self.position;
                while self
                    .source
                    .as_bytes()
                    .get(self.position)
                    .is_some_and(|byte| !byte.is_ascii_whitespace())
                {
                    self.position += 1;
                }
                let (name, ty) = parse_named_type(&self.source[start..self.position], line)?;
                Ok(Parameter::Field { name, ty })
            }
        }
    }

    fn take_delimited(
        &mut self,
        open: u8,
        close: u8,
        line: usize,
    ) -> Result<&'source str, SchemaParseError> {
        debug_assert_eq!(self.source.as_bytes()[self.position], open);
        let start = self.position + 1;
        let Some(relative_end) = self.source.as_bytes()[start..]
            .iter()
            .position(|byte| *byte == close)
        else {
            return Err(parse_error(
                SchemaParseErrorKind::InvalidParameter,
                line,
                format!("unclosed `{}` parameter", char::from(open)),
            ));
        };
        let end = start + relative_end;
        self.position = end + 1;
        self.require_boundary(line)?;
        Ok(&self.source[start..end])
    }

    fn require_boundary(&self, line: usize) -> Result<(), SchemaParseError> {
        if self
            .source
            .as_bytes()
            .get(self.position)
            .is_some_and(|byte| !byte.is_ascii_whitespace())
        {
            return Err(parse_error(
                SchemaParseErrorKind::InvalidParameter,
                line,
                "special parameter must end at whitespace",
            ));
        }
        Ok(())
    }
}

fn parse_named_type(source: &str, line: usize) -> Result<(String, TypeRef), SchemaParseError> {
    let Some((name, type_source)) = source.split_once(':') else {
        return Err(parse_error(
            SchemaParseErrorKind::InvalidParameter,
            line,
            format!("parameter `{source}` has no type separator"),
        ));
    };
    if type_source.contains('#')
        || type_source.contains('?')
        || type_source.contains('.')
        || type_source.contains('%')
        || type_source.contains('!')
    {
        return Err(parse_error(
            SchemaParseErrorKind::UnsupportedSyntax,
            line,
            format!("conditional or decorated parameter `{source}` is unsupported"),
        ));
    }
    if type_source.contains(':') || !is_lowercase_identifier(name) {
        return Err(parse_error(
            SchemaParseErrorKind::InvalidParameter,
            line,
            format!("invalid parameter `{source}`"),
        ));
    }
    let ty = parse_type(type_source, false, line)?;
    Ok((name.to_owned(), ty))
}

fn parse_type(
    source: &str,
    allow_application: bool,
    line: usize,
) -> Result<TypeRef, SchemaParseError> {
    let mut cursor = TypeCursor::new(source);
    let mut result = cursor.parse_primary(line, 0)?;
    cursor.skip_whitespace();
    if allow_application {
        while !cursor.is_finished() {
            result.arguments.push(cursor.parse_primary(line, 0)?);
            cursor.skip_whitespace();
        }
    } else if !cursor.is_finished() {
        return Err(invalid_type(line, source));
    }
    Ok(result)
}

struct TypeCursor<'source> {
    source: &'source str,
    position: usize,
}

impl<'source> TypeCursor<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn parse_primary(&mut self, line: usize, depth: usize) -> Result<TypeRef, SchemaParseError> {
        if depth > MAX_TYPE_NESTING {
            return Err(parse_error(
                SchemaParseErrorKind::TypeNestingLimit,
                line,
                format!("type nesting exceeds {MAX_TYPE_NESTING}"),
            ));
        }
        self.skip_whitespace();
        let start = self.position;
        while self
            .source
            .as_bytes()
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            self.position += 1;
        }
        let name = &self.source[start..self.position];
        if !is_type_identifier(name) {
            return Err(invalid_type(line, self.source));
        }

        self.skip_whitespace();
        let mut arguments = Vec::new();
        if self.peek() == Some(b'<') {
            self.position += 1;
            loop {
                arguments.push(self.parse_primary(line, depth + 1)?);
                self.skip_whitespace();
                match self.peek() {
                    Some(b',') => self.position += 1,
                    Some(b'>') => {
                        self.position += 1;
                        break;
                    }
                    _ => return Err(invalid_type(line, self.source)),
                }
            }
        }
        Ok(TypeRef {
            name: name.to_owned(),
            arguments,
        })
    }

    fn skip_whitespace(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.position).copied()
    }

    fn is_finished(&self) -> bool {
        self.position == self.source.len()
    }
}

fn invalid_type(line: usize, source: &str) -> SchemaParseError {
    parse_error(
        SchemaParseErrorKind::InvalidType,
        line,
        format!("invalid type `{source}`"),
    )
}

fn is_lowercase_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_type_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_boxed_result_identifier(value: &str) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase())
        && is_type_identifier(value)
        && !is_reserved_type_name(value)
}

fn is_reserved_type_name(value: &str) -> bool {
    matches!(value, "Final" | "New" | "Empty")
}

fn parse_error(
    kind: SchemaParseErrorKind,
    line: usize,
    detail: impl Into<String>,
) -> SchemaParseError {
    SchemaParseError::new(kind, line, detail)
}

#[cfg(test)]
mod tests;
