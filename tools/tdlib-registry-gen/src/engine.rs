use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use telegram_core::feature::FeatureId;
use telegram_core::schema::{Definition, Schema};

const FORMAT_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
const MAX_POLICY_BYTES: usize = 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_METHODS: usize = 2_048;
const MAX_RULES: usize = 22;
const MAX_ATOMS: usize = 512;
const MAX_OVERRIDES: usize = 1_024;
const MAX_ATOM_BYTES: usize = 64;
const MAX_RATIONALE_BYTES: usize = 1_024;
const MAX_EXAMPLES_PER_RULE: usize = 32;
const GENERIC_PREFIXES: [&str; 12] = [
    "add", "check", "create", "delete", "edit", "get", "remove", "search", "send", "set", "toggle",
    "update",
];

pub fn generate(
    manifest_bytes: &[u8],
    schema_bytes: &[u8],
    policy_bytes: &[u8],
) -> Result<Vec<u8>, GenerationError> {
    enforce_input_cap("vendor manifest", manifest_bytes.len(), MAX_MANIFEST_BYTES)?;
    enforce_input_cap("TDLib schema", schema_bytes.len(), MAX_SCHEMA_BYTES)?;
    enforce_input_cap("owner policy", policy_bytes.len(), MAX_POLICY_BYTES)?;

    let manifest: VendorManifest = parse_json(manifest_bytes, "vendor manifest")?;
    let schema_source = std::str::from_utf8(schema_bytes).map_err(|error| {
        GenerationError::new(
            GenerationErrorKind::InvalidSchema,
            format!("TDLib schema is not UTF-8: {error}"),
        )
    })?;
    let schema = Schema::parse(schema_source).map_err(|error| {
        GenerationError::new(GenerationErrorKind::InvalidSchema, error.to_string())
    })?;
    validate_manifest(&manifest, schema_bytes, &schema)?;

    let policy_dto: PolicyDto = parse_json(policy_bytes, "owner policy")?;
    let policy = Policy::try_from_dto(policy_dto, &manifest.schema.sha256)?;
    classify(manifest_bytes, schema_bytes, manifest, schema, policy)
}

fn classify(
    manifest_bytes: &[u8],
    schema_bytes: &[u8],
    manifest: VendorManifest,
    schema: Schema,
    policy: Policy,
) -> Result<Vec<u8>, GenerationError> {
    let mut methods: Vec<&Definition> = schema.methods().iter().collect();
    methods.sort_unstable_by_key(|method| method.name());
    if methods.len() > MAX_METHODS {
        return Err(GenerationError::new(
            GenerationErrorKind::ResourceLimit,
            format!(
                "schema has {} methods, exceeding the {MAX_METHODS}-method cap",
                methods.len()
            ),
        ));
    }

    let by_name: BTreeMap<&str, &Definition> = methods
        .iter()
        .map(|method| (method.name(), *method))
        .collect();
    let mut candidates: BTreeMap<&str, BTreeSet<FeatureId>> = methods
        .iter()
        .map(|method| (method.name(), BTreeSet::new()))
        .collect();

    for rule in &policy.rules {
        let matched = match_rule(rule, &methods)?;
        validate_rule_evidence(rule, &matched, &by_name)?;
        for method in matched {
            candidates
                .get_mut(method.name())
                .expect("candidate map is built from the same methods")
                .insert(rule.feature_id);
        }
    }

    let overrides: BTreeMap<&str, &OwnershipOverride> = policy
        .overrides
        .iter()
        .map(|item| (item.method.as_str(), item))
        .collect();
    let mut consumed_overrides = BTreeSet::new();
    let mut winner_counts = BTreeMap::<FeatureId, usize>::new();
    let mut rows = Vec::with_capacity(methods.len());

    for method in methods {
        let actual_candidates = candidates
            .get(method.name())
            .expect("every parsed method has a candidate set");
        let override_entry = overrides.get(method.name()).copied();
        let (owner, owner_source) = resolve_owner(method, actual_candidates, override_entry)?;
        if override_entry.is_some() {
            consumed_overrides.insert(method.name());
        }
        *winner_counts.entry(owner).or_default() += 1;

        rows.push(MethodRow {
            method: method.name().to_owned(),
            canonical_signature: method.canonical_signature().to_owned(),
            signature_sha256: sha256_hex(method.canonical_signature().as_bytes()),
            source_line: method.line(),
            feature_id: owner.as_str(),
            owner_source,
            candidates: actual_candidates
                .iter()
                .map(|candidate| candidate.as_str())
                .collect(),
        });
    }

    if consumed_overrides.len() != policy.overrides.len() {
        let stale = policy
            .overrides
            .iter()
            .find(|item| !consumed_overrides.contains(item.method.as_str()))
            .expect("different set lengths imply an unconsumed override");
        return Err(GenerationError::new(
            GenerationErrorKind::StaleOverride,
            format!("override for unknown method {:?}", stale.method),
        ));
    }

    for rule in &policy.rules {
        if winner_counts.get(&rule.feature_id).copied().unwrap_or(0) == 0 {
            return Err(GenerationError::new(
                GenerationErrorKind::InvalidPolicy,
                format!(
                    "rule {} does not own any method after overrides",
                    rule.feature_id
                ),
            ));
        }
    }

    let semantic_policy = policy.canonical_bytes()?;
    let rule_bytes = policy.canonical_rule_bytes()?;
    let override_bytes = policy.canonical_override_bytes()?;
    let mapping_bytes = compact_json(&rows, "method mapping")?;
    let features = feature_summaries(&rows);
    let output = GeneratedManifest {
        format_version: FORMAT_VERSION,
        generated_by: "tdlib-registry-gen",
        engine_source_sha256: sha256_hex(include_bytes!("engine.rs")),
        vendor_manifest_sha256: sha256_hex(manifest_bytes),
        schema: SchemaEvidence {
            repository: manifest.upstream.repository,
            commit: manifest.upstream.commit,
            version: manifest.upstream.version,
            sha256: sha256_hex(schema_bytes),
            bytes: schema_bytes.len(),
            definitions: schema.definitions().len(),
            methods: schema.methods().len(),
            updates: schema.inventory().update_names().len(),
            authorization_states: schema.inventory().authorization_state_names().len(),
        },
        policy: PolicyEvidence {
            semantic_sha256: sha256_hex(&semantic_policy),
            rules_sha256: sha256_hex(&rule_bytes),
            overrides_sha256: sha256_hex(&override_bytes),
        },
        counts: Counts {
            schema_methods: rows.len(),
            owned_methods: rows.len(),
            rules: policy.rules.len(),
            overrides: policy.overrides.len(),
        },
        mapping_sha256: sha256_hex(&mapping_bytes),
        features,
        methods: rows,
    };

    let mut encoded = serde_json::to_vec_pretty(&output).map_err(|error| {
        GenerationError::new(
            GenerationErrorKind::Serialization,
            format!("cannot serialize owner manifest: {error}"),
        )
    })?;
    encoded.push(b'\n');
    if encoded.len() > MAX_OUTPUT_BYTES {
        return Err(GenerationError::new(
            GenerationErrorKind::ResourceLimit,
            format!(
                "generated manifest is {} bytes, exceeding the {MAX_OUTPUT_BYTES}-byte cap",
                encoded.len()
            ),
        ));
    }
    Ok(encoded)
}

fn match_rule<'schema>(
    rule: &FeatureRule,
    methods: &[&'schema Definition],
) -> Result<Vec<&'schema Definition>, GenerationError> {
    let mut atom_used = vec![false; rule.atoms.len()];
    let matched = methods
        .iter()
        .copied()
        .filter(|method| {
            let mut any = false;
            for (index, atom) in rule.atoms.iter().enumerate() {
                if atom.matches(method.name()) {
                    atom_used[index] = true;
                    any = true;
                }
            }
            any
        })
        .collect::<Vec<_>>();

    if let Some((index, _)) = atom_used.iter().enumerate().find(|(_, used)| !**used) {
        return Err(GenerationError::new(
            GenerationErrorKind::InvalidPolicy,
            format!(
                "dead {:?} atom {:?} in rule {}",
                rule.atoms[index].kind, rule.atoms[index].value, rule.feature_id
            ),
        ));
    }
    if matched.is_empty() {
        return Err(GenerationError::new(
            GenerationErrorKind::InvalidPolicy,
            format!("rule {} does not match a schema method", rule.feature_id),
        ));
    }
    Ok(matched)
}

fn validate_rule_evidence(
    rule: &FeatureRule,
    matched: &[&Definition],
    methods: &BTreeMap<&str, &Definition>,
) -> Result<(), GenerationError> {
    let actual_hash = method_set_sha256(matched.iter().copied());
    if matched.len() != rule.expected.method_count || actual_hash != rule.expected.method_set_sha256
    {
        return Err(GenerationError::new(
            GenerationErrorKind::RuleDrift,
            format!(
                "rule {} expected {} methods with hash {}, got {} with hash {actual_hash}",
                rule.feature_id,
                rule.expected.method_count,
                rule.expected.method_set_sha256,
                matched.len()
            ),
        ));
    }

    let matched_names: BTreeSet<_> = matched.iter().map(|method| method.name()).collect();
    for example in &rule.positive_examples {
        if !methods.contains_key(example.as_str()) || !matched_names.contains(example.as_str()) {
            return Err(GenerationError::new(
                GenerationErrorKind::InvalidPolicy,
                format!(
                    "positive example {example:?} is not matched by rule {}",
                    rule.feature_id
                ),
            ));
        }
    }
    for example in &rule.negative_examples {
        if !methods.contains_key(example.as_str()) || matched_names.contains(example.as_str()) {
            return Err(GenerationError::new(
                GenerationErrorKind::InvalidPolicy,
                format!(
                    "negative example {example:?} is missing or matched by rule {}",
                    rule.feature_id
                ),
            ));
        }
    }
    Ok(())
}

fn resolve_owner(
    method: &Definition,
    candidates: &BTreeSet<FeatureId>,
    override_entry: Option<&OwnershipOverride>,
) -> Result<(FeatureId, &'static str), GenerationError> {
    match candidates.len() {
        0 => Err(GenerationError::new(
            GenerationErrorKind::Coverage,
            format!("method {:?} has no owner candidate", method.name()),
        )),
        1 => {
            if override_entry.is_some() {
                return Err(GenerationError::new(
                    GenerationErrorKind::StaleOverride,
                    format!(
                        "override for {:?} is redundant because it has one candidate",
                        method.name()
                    ),
                ));
            }
            Ok((*candidates.iter().next().expect("one candidate"), "rule"))
        }
        _ => {
            let override_entry = override_entry.ok_or_else(|| {
                GenerationError::new(
                    GenerationErrorKind::Coverage,
                    format!(
                        "method {:?} has ambiguous candidates {}",
                        method.name(),
                        format_features(candidates)
                    ),
                )
            })?;
            let expected: BTreeSet<_> =
                override_entry.expected_candidates.iter().copied().collect();
            if expected != *candidates || !candidates.contains(&override_entry.owner) {
                return Err(GenerationError::new(
                    GenerationErrorKind::StaleOverride,
                    format!(
                        "override for {:?} expected {}, actual {}",
                        method.name(),
                        format_features(&expected),
                        format_features(candidates)
                    ),
                ));
            }
            let actual_signature = sha256_hex(method.canonical_signature().as_bytes());
            if actual_signature != override_entry.signature_sha256 {
                return Err(GenerationError::new(
                    GenerationErrorKind::StaleOverride,
                    format!("override for {:?} has stale signature hash", method.name()),
                ));
            }
            Ok((override_entry.owner, "override"))
        }
    }
}

fn feature_summaries(rows: &[MethodRow]) -> Vec<FeatureSummary> {
    FeatureId::ALL
        .iter()
        .map(|feature_id| {
            let owned: Vec<_> = rows
                .iter()
                .filter(|row| row.feature_id == feature_id.as_str())
                .collect();
            FeatureSummary {
                feature_id: feature_id.as_str(),
                method_count: owned.len(),
                method_set_sha256: row_set_sha256(&owned),
            }
        })
        .collect()
}

fn method_set_sha256<'a>(methods: impl IntoIterator<Item = &'a Definition>) -> String {
    let mut methods = methods.into_iter().collect::<Vec<_>>();
    methods.sort_unstable_by_key(|method| method.name());
    let mut hasher = Sha256::new();
    for method in methods {
        hasher.update(method.name().as_bytes());
        hasher.update([0]);
        hasher.update(sha256_hex(method.canonical_signature().as_bytes()).as_bytes());
        hasher.update(b"\n");
    }
    digest_hex(hasher.finalize())
}

fn row_set_sha256(rows: &[&MethodRow]) -> String {
    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(row.method.as_bytes());
        hasher.update([0]);
        hasher.update(row.signature_sha256.as_bytes());
        hasher.update(b"\n");
    }
    digest_hex(hasher.finalize())
}

fn validate_manifest(
    manifest: &VendorManifest,
    schema_bytes: &[u8],
    schema: &Schema,
) -> Result<(), GenerationError> {
    if manifest.format_version != FORMAT_VERSION {
        return Err(GenerationError::new(
            GenerationErrorKind::InvalidManifest,
            format!(
                "unsupported vendor manifest format_version {}",
                manifest.format_version
            ),
        ));
    }
    validate_nonempty("upstream.repository", &manifest.upstream.repository, 512)
        .map_err(GenerationError::invalid_manifest)?;
    validate_ascii_hex("upstream.commit", &manifest.upstream.commit, 40)
        .map_err(GenerationError::invalid_manifest)?;
    validate_nonempty("upstream.version", &manifest.upstream.version, 64)
        .map_err(GenerationError::invalid_manifest)?;
    validate_ascii_hex("schema.sha256", &manifest.schema.sha256, 64)
        .map_err(GenerationError::invalid_manifest)?;

    let inventory = schema.inventory();
    let expected_hash = sha256_hex(schema_bytes);
    let counts_match = manifest.schema.definitions == schema.definitions().len()
        && manifest.schema.functions == schema.methods().len()
        && manifest.schema.updates == inventory.update_names().len()
        && manifest.schema.authorization_states == inventory.authorization_state_names().len();
    let bytes_match = manifest
        .schema
        .bytes
        .is_none_or(|expected| expected == schema_bytes.len());
    if manifest.schema.sha256 != expected_hash || !counts_match || !bytes_match {
        return Err(GenerationError::new(
            GenerationErrorKind::SchemaDrift,
            "vendor manifest does not describe the supplied TDLib schema",
        ));
    }
    Ok(())
}

fn enforce_input_cap(name: &str, actual: usize, limit: usize) -> Result<(), GenerationError> {
    if actual > limit {
        return Err(GenerationError::new(
            GenerationErrorKind::ResourceLimit,
            format!("{name} is {actual} bytes, exceeding the {limit}-byte cap"),
        ));
    }
    Ok(())
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    name: &str,
) -> Result<T, GenerationError> {
    serde_json::from_slice(bytes).map_err(|error| {
        let kind = if name == "vendor manifest" {
            GenerationErrorKind::InvalidManifest
        } else {
            GenerationErrorKind::InvalidPolicy
        };
        GenerationError::new(kind, format!("invalid {name}: {error}"))
    })
}

fn compact_json<T: Serialize>(value: &T, name: &str) -> Result<Vec<u8>, GenerationError> {
    serde_json::to_vec(value).map_err(|error| {
        GenerationError::new(
            GenerationErrorKind::Serialization,
            format!("cannot serialize canonical {name}: {error}"),
        )
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    digest_hex(Sha256::digest(bytes))
}

fn digest_hex(digest: impl AsRef<[u8]>) -> String {
    let bytes = digest.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn validate_ascii_hex(name: &str, value: &str, expected_len: usize) -> Result<(), String> {
    if value.len() != expected_len
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{name} must be {expected_len} lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn validate_nonempty(name: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    if value.trim().is_empty() || value != value.trim() || value.len() > max_bytes {
        return Err(format!(
            "{name} must be non-empty, trimmed, and at most {max_bytes} bytes"
        ));
    }
    Ok(())
}

fn format_features(features: &BTreeSet<FeatureId>) -> String {
    features
        .iter()
        .map(|feature| feature.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VendorManifest {
    format_version: u32,
    upstream: Upstream,
    #[serde(default, rename = "cmake")]
    _cmake: Option<serde::de::IgnoredAny>,
    schema: ManifestSchema,
    #[serde(default, rename = "license")]
    _license: Option<serde::de::IgnoredAny>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Upstream {
    repository: String,
    commit: String,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestSchema {
    #[serde(default, rename = "source_path")]
    _source_path: Option<serde::de::IgnoredAny>,
    #[serde(default, rename = "vendored_path")]
    _vendored_path: Option<serde::de::IgnoredAny>,
    sha256: String,
    #[serde(default)]
    bytes: Option<usize>,
    definitions: usize,
    functions: usize,
    updates: usize,
    authorization_states: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyDto {
    format_version: u32,
    schema_sha256: String,
    rules: Vec<FeatureRuleDto>,
    overrides: Vec<OwnershipOverrideDto>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FeatureRuleDto {
    feature_id: String,
    any: Vec<NameAtom>,
    expected: ExpectedSet,
    positive_examples: Vec<String>,
    negative_examples: Vec<String>,
    rationale: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct NameAtom {
    kind: AtomKind,
    value: String,
}

impl NameAtom {
    fn matches(&self, method: &str) -> bool {
        match self.kind {
            AtomKind::Exact => method == self.value,
            AtomKind::Prefix => method.starts_with(&self.value),
            AtomKind::Contains => method.contains(&self.value),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.value.is_empty()
            || self.value.len() > MAX_ATOM_BYTES
            || !self.value.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(format!(
                "{:?} atom {:?} must contain 1..={MAX_ATOM_BYTES} ASCII alphanumeric bytes",
                self.kind, self.value
            ));
        }
        if self.kind == AtomKind::Prefix && GENERIC_PREFIXES.contains(&self.value.as_str()) {
            return Err(format!(
                "generic prefix {:?} is forbidden; F020 is not a fallback",
                self.value
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum AtomKind {
    Exact,
    Prefix,
    Contains,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedSet {
    method_count: usize,
    method_set_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnershipOverrideDto {
    method: String,
    owner: String,
    expected_candidates: Vec<String>,
    signature_sha256: String,
    rationale: String,
}

#[derive(Debug)]
struct Policy {
    schema_sha256: String,
    rules: Vec<FeatureRule>,
    overrides: Vec<OwnershipOverride>,
}

impl Policy {
    fn try_from_dto(dto: PolicyDto, schema_sha256: &str) -> Result<Self, GenerationError> {
        if dto.format_version != FORMAT_VERSION {
            return Err(GenerationError::invalid_policy(format!(
                "unsupported policy format_version {}",
                dto.format_version
            )));
        }
        validate_ascii_hex("schema_sha256", &dto.schema_sha256, 64)
            .map_err(GenerationError::invalid_policy)?;
        if dto.schema_sha256 != schema_sha256 {
            return Err(GenerationError::new(
                GenerationErrorKind::SchemaDrift,
                "owner policy is bound to a different schema hash",
            ));
        }
        if dto.rules.is_empty() || dto.rules.len() > MAX_RULES {
            return Err(GenerationError::invalid_policy(format!(
                "policy must contain 1..={MAX_RULES} feature rules"
            )));
        }
        if dto.overrides.len() > MAX_OVERRIDES {
            return Err(GenerationError::new(
                GenerationErrorKind::ResourceLimit,
                format!(
                    "policy has {} overrides, exceeding the {MAX_OVERRIDES}-override cap",
                    dto.overrides.len()
                ),
            ));
        }

        let mut rules = Vec::with_capacity(dto.rules.len());
        let mut seen_features = BTreeSet::new();
        let mut atom_count = 0_usize;
        for dto_rule in dto.rules {
            let feature_id = FeatureId::try_from(dto_rule.feature_id.as_str())
                .map_err(|error| GenerationError::invalid_policy(error.to_string()))?;
            if !seen_features.insert(feature_id) {
                return Err(GenerationError::invalid_policy(format!(
                    "duplicate feature rule for {feature_id}"
                )));
            }
            if dto_rule.any.is_empty() {
                return Err(GenerationError::invalid_policy(format!(
                    "rule {feature_id} has no positive atoms"
                )));
            }
            atom_count = atom_count.saturating_add(dto_rule.any.len());
            if atom_count > MAX_ATOMS {
                return Err(GenerationError::new(
                    GenerationErrorKind::ResourceLimit,
                    format!("policy exceeds the {MAX_ATOMS}-atom cap"),
                ));
            }

            let mut atoms = dto_rule.any;
            atoms.sort_unstable();
            for atom in &atoms {
                atom.validate().map_err(GenerationError::invalid_policy)?;
            }
            if atoms.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(GenerationError::invalid_policy(format!(
                    "rule {feature_id} contains a duplicate atom"
                )));
            }
            validate_ascii_hex(
                "expected.method_set_sha256",
                &dto_rule.expected.method_set_sha256,
                64,
            )
            .map_err(GenerationError::invalid_policy)?;
            if dto_rule.expected.method_count == 0 || dto_rule.expected.method_count > MAX_METHODS {
                return Err(GenerationError::invalid_policy(format!(
                    "rule {feature_id} has an invalid expected method count"
                )));
            }
            let positive_examples =
                canonical_examples(feature_id, "positive_examples", dto_rule.positive_examples)?;
            let negative_examples =
                canonical_examples(feature_id, "negative_examples", dto_rule.negative_examples)?;
            validate_nonempty("rule rationale", &dto_rule.rationale, MAX_RATIONALE_BYTES)
                .map_err(GenerationError::invalid_policy)?;

            rules.push(FeatureRule {
                feature_id,
                atoms,
                expected: dto_rule.expected,
                positive_examples,
                negative_examples,
                rationale: dto_rule.rationale,
            });
        }
        rules.sort_unstable_by_key(|rule| rule.feature_id);

        let mut overrides = Vec::with_capacity(dto.overrides.len());
        let mut seen_methods = BTreeSet::new();
        for dto_override in dto.overrides {
            validate_method_name("override method", &dto_override.method)
                .map_err(GenerationError::invalid_policy)?;
            if !seen_methods.insert(dto_override.method.clone()) {
                return Err(GenerationError::invalid_policy(format!(
                    "duplicate override for {:?}",
                    dto_override.method
                )));
            }
            let owner = FeatureId::try_from(dto_override.owner.as_str())
                .map_err(|error| GenerationError::invalid_policy(error.to_string()))?;
            let mut expected_candidates = dto_override
                .expected_candidates
                .iter()
                .map(|candidate| {
                    FeatureId::try_from(candidate.as_str())
                        .map_err(|error| GenerationError::invalid_policy(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            expected_candidates.sort_unstable();
            if expected_candidates.is_empty()
                || expected_candidates
                    .windows(2)
                    .any(|pair| pair[0] == pair[1])
            {
                return Err(GenerationError::invalid_policy(format!(
                    "override for {:?} must pin unique candidates",
                    dto_override.method
                )));
            }
            if !expected_candidates.contains(&owner) {
                return Err(GenerationError::invalid_policy(format!(
                    "override owner {owner} is absent from expected candidates"
                )));
            }
            validate_ascii_hex(
                "override signature_sha256",
                &dto_override.signature_sha256,
                64,
            )
            .map_err(GenerationError::invalid_policy)?;
            validate_nonempty(
                "override rationale",
                &dto_override.rationale,
                MAX_RATIONALE_BYTES,
            )
            .map_err(GenerationError::invalid_policy)?;

            overrides.push(OwnershipOverride {
                method: dto_override.method,
                owner,
                expected_candidates,
                signature_sha256: dto_override.signature_sha256,
                rationale: dto_override.rationale,
            });
        }
        overrides.sort_unstable_by(|left, right| left.method.cmp(&right.method));

        Ok(Self {
            schema_sha256: dto.schema_sha256,
            rules,
            overrides,
        })
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, GenerationError> {
        compact_json(
            &CanonicalPolicy {
                format_version: FORMAT_VERSION,
                schema_sha256: &self.schema_sha256,
                rules: self.canonical_rules(),
                overrides: self.canonical_overrides(),
            },
            "policy",
        )
    }

    fn canonical_rule_bytes(&self) -> Result<Vec<u8>, GenerationError> {
        compact_json(&self.canonical_rules(), "rules")
    }

    fn canonical_override_bytes(&self) -> Result<Vec<u8>, GenerationError> {
        compact_json(&self.canonical_overrides(), "overrides")
    }

    fn canonical_rules(&self) -> Vec<CanonicalRule<'_>> {
        self.rules
            .iter()
            .map(|rule| CanonicalRule {
                feature_id: rule.feature_id.as_str(),
                any: &rule.atoms,
                expected: &rule.expected,
                positive_examples: &rule.positive_examples,
                negative_examples: &rule.negative_examples,
                rationale: &rule.rationale,
            })
            .collect()
    }

    fn canonical_overrides(&self) -> Vec<CanonicalOverride<'_>> {
        self.overrides
            .iter()
            .map(|item| CanonicalOverride {
                method: &item.method,
                owner: item.owner.as_str(),
                expected_candidates: item
                    .expected_candidates
                    .iter()
                    .map(|feature| feature.as_str())
                    .collect(),
                signature_sha256: &item.signature_sha256,
                rationale: &item.rationale,
            })
            .collect()
    }
}

fn canonical_examples(
    feature_id: FeatureId,
    name: &str,
    mut values: Vec<String>,
) -> Result<Vec<String>, GenerationError> {
    if values.is_empty() || values.len() > MAX_EXAMPLES_PER_RULE {
        return Err(GenerationError::invalid_policy(format!(
            "rule {feature_id} must contain 1..={MAX_EXAMPLES_PER_RULE} {name}"
        )));
    }
    for value in &values {
        validate_method_name(name, value).map_err(GenerationError::invalid_policy)?;
    }
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(GenerationError::invalid_policy(format!(
            "rule {feature_id} contains duplicate {name}"
        )));
    }
    Ok(values)
}

fn validate_method_name(name: &str, value: &str) -> Result<(), String> {
    let mut bytes = value.bytes();
    let first = bytes.next();
    if value.len() > MAX_ATOM_BYTES
        || !first.is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(format!(
            "{name} {value:?} is not a bounded lower-camel method name"
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct FeatureRule {
    feature_id: FeatureId,
    atoms: Vec<NameAtom>,
    expected: ExpectedSet,
    positive_examples: Vec<String>,
    negative_examples: Vec<String>,
    rationale: String,
}

#[derive(Debug)]
struct OwnershipOverride {
    method: String,
    owner: FeatureId,
    expected_candidates: Vec<FeatureId>,
    signature_sha256: String,
    rationale: String,
}

#[derive(Serialize)]
struct CanonicalPolicy<'a> {
    format_version: u32,
    schema_sha256: &'a str,
    rules: Vec<CanonicalRule<'a>>,
    overrides: Vec<CanonicalOverride<'a>>,
}

#[derive(Serialize)]
struct CanonicalRule<'a> {
    feature_id: &'static str,
    any: &'a [NameAtom],
    expected: &'a ExpectedSet,
    positive_examples: &'a [String],
    negative_examples: &'a [String],
    rationale: &'a str,
}

#[derive(Serialize)]
struct CanonicalOverride<'a> {
    method: &'a str,
    owner: &'static str,
    expected_candidates: Vec<&'static str>,
    signature_sha256: &'a str,
    rationale: &'a str,
}

#[derive(Serialize)]
struct GeneratedManifest {
    format_version: u32,
    generated_by: &'static str,
    engine_source_sha256: String,
    vendor_manifest_sha256: String,
    schema: SchemaEvidence,
    policy: PolicyEvidence,
    counts: Counts,
    mapping_sha256: String,
    features: Vec<FeatureSummary>,
    methods: Vec<MethodRow>,
}

#[derive(Serialize)]
struct SchemaEvidence {
    repository: String,
    commit: String,
    version: String,
    sha256: String,
    bytes: usize,
    definitions: usize,
    methods: usize,
    updates: usize,
    authorization_states: usize,
}

#[derive(Serialize)]
struct PolicyEvidence {
    semantic_sha256: String,
    rules_sha256: String,
    overrides_sha256: String,
}

#[derive(Serialize)]
struct Counts {
    schema_methods: usize,
    owned_methods: usize,
    rules: usize,
    overrides: usize,
}

#[derive(Serialize)]
struct FeatureSummary {
    feature_id: &'static str,
    method_count: usize,
    method_set_sha256: String,
}

#[derive(Serialize)]
struct MethodRow {
    method: String,
    canonical_signature: String,
    signature_sha256: String,
    source_line: usize,
    feature_id: &'static str,
    owner_source: &'static str,
    candidates: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationErrorKind {
    ResourceLimit,
    InvalidManifest,
    InvalidSchema,
    SchemaDrift,
    InvalidPolicy,
    RuleDrift,
    StaleOverride,
    Coverage,
    Serialization,
}

#[derive(Debug)]
pub struct GenerationError {
    kind: GenerationErrorKind,
    detail: String,
}

impl GenerationError {
    fn new(kind: GenerationErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    fn invalid_policy(detail: impl Into<String>) -> Self {
        Self::new(GenerationErrorKind::InvalidPolicy, detail)
    }

    fn invalid_manifest(detail: impl Into<String>) -> Self {
        Self::new(GenerationErrorKind::InvalidManifest, detail)
    }

    pub fn kind(&self) -> GenerationErrorKind {
        self.kind
    }
}

impl fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for GenerationError {}

#[cfg(test)]
mod corpus_tests;
#[cfg(test)]
mod tests;
