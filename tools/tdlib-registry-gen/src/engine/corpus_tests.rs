use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};
use telegram_core::feature::FeatureId;
use telegram_core::schema::{Definition, Schema};

use super::generate;

const EXPECTED_MAPPING_SHA256: &str =
    "4687dc47c69e798e0afb9e0ea8ee82ed5d91bcde00e51f476ff08ea8226ce790";
const EXPECTED_OWNER_ASSIGNMENT_SHA256: &str =
    "72c741dc024091b5f25f5fc97a46224a557560dcb483bb0d3a29c4085f9b13ac";
const EXPECTED_POLICY_SEMANTIC_SHA256: &str =
    "7b1f2bff6853e878522eb069466b1f42b6480409c1e1ecdca521def9ef96d9d8";
const EXPECTED_RULES_SHA256: &str =
    "cbec907cc3d97b0e93df90dbcc2568e186cfd484e31e619e26c19c9450ce96d4";
const EXPECTED_OVERRIDES_SHA256: &str =
    "96f238dcef04e4850c852a1349915a8ee5b61c4dbb52e0a035679a7fe2250d50";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const FEATURE_ORACLES: [(&str, usize, &str); 22] = [
    (
        "F001",
        3,
        "7d77443f1cc8d85b3ab18d244574db60932f68d706f07481083cbfa5a730e5d6",
    ),
    (
        "F002",
        25,
        "af05c082d3cc7baf64ef793b16a1efa1f1b8e0c1f754e912f2a8898913b29021",
    ),
    ("F003", 0, EMPTY_SHA256),
    (
        "F004",
        1,
        "fd2280b827054de69401f5a95793ed19c5412c8e009208f11080d589614a385d",
    ),
    ("F005", 0, EMPTY_SHA256),
    ("F006", 0, EMPTY_SHA256),
    (
        "F007",
        50,
        "2a305754de0ca9d813edf6dbaaef754c8b5d877a91fb2b7fd33fc288452a8244",
    ),
    (
        "F008",
        74,
        "b0aa81632edd3792a6e9c7e01ee62eb38af6761227162ebe32faaa6a55146a21",
    ),
    (
        "F009",
        156,
        "23d5ef0b8e4abf2f6ebce9d6d34d1502b4a5005757e2c71ecc1b44b5775f06f3",
    ),
    (
        "F010",
        32,
        "f247295b1dc84e45bcd4cc5df05ef1e7e116164c02d20d1749fd47215f251149",
    ),
    (
        "F011",
        92,
        "207c6349293ddaf0d5a741f34700ef6c193c13f399c2396baae4b64b536be1a8",
    ),
    (
        "F012",
        63,
        "f31f4647e76814e44b70241fd2fa4f36dc796d0b528f648a01e197829e5e1078",
    ),
    (
        "F013",
        23,
        "9efa3bde6e1c966773b3eb179949e77a6118cda7e70b551dc93ec59c8414da87",
    ),
    (
        "F014",
        72,
        "70784511b8ec9ae7297c7699a01a0d5d34001e5dde9dd305518af6d0cd16541b",
    ),
    (
        "F015",
        99,
        "06e3c210c43fabd4dc59da8e623137ccc4add6418779c7981b036c35db1f4054",
    ),
    (
        "F016",
        63,
        "352d66d679e51f4aa7247caee9ace55188918cb3a267693ca439e3483d9b6c02",
    ),
    (
        "F017",
        49,
        "c4b9399382ff771564e6dd975800fad9fe38141341e1702f6f4d835cca311745",
    ),
    (
        "F018",
        105,
        "cba3481722ee1b616619d37a6ef9fda1aa9e56526b3d3d393d793e8f0ab4a6c3",
    ),
    (
        "F019",
        17,
        "4ba29528d0f34648e4090df200e938390729d59c45c0d29f04cddd75e7e08e3e",
    ),
    (
        "F020",
        86,
        "31196a63c7c10e10834537742f2785121fe9c7b384bc5d6fdec0d345d2e58d7e",
    ),
    ("F021", 0, EMPTY_SHA256),
    ("F022", 0, EMPTY_SHA256),
];

#[test]
fn committed_owner_artifact_exactly_covers_the_pinned_method_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read(root.join("vendor/tdlib/manifest.json")).expect("vendor manifest");
    let schema = fs::read(root.join("vendor/tdlib/td_api.tl")).expect("pinned schema");
    let policy =
        fs::read(root.join("policy/tdlib-feature-owners.json")).expect("reviewed owner policy");
    let committed = fs::read(root.join("generated/tdlib-feature-owners.json"))
        .expect("generated owner artifact");

    let expected = generate(&manifest, &schema, &policy).expect("complete owner classification");
    assert_eq!(committed, expected, "committed artifact must be canonical");

    let artifact: Value = serde_json::from_slice(&committed).expect("owner artifact JSON");
    assert_eq!(
        object_keys(&artifact),
        BTreeSet::from([
            "counts",
            "engine_source_sha256",
            "features",
            "format_version",
            "generated_by",
            "mapping_sha256",
            "methods",
            "policy",
            "schema",
            "vendor_manifest_sha256",
        ]),
        "owner artifact must not claim capability/risk/retry/codec/router parity"
    );
    assert_eq!(artifact["format_version"], 1);
    assert_eq!(artifact["generated_by"], "tdlib-registry-gen");
    assert_eq!(
        object_keys(&artifact["counts"]),
        BTreeSet::from(["overrides", "owned_methods", "rules", "schema_methods"])
    );
    assert_eq!(
        object_keys(&artifact["policy"]),
        BTreeSet::from(["overrides_sha256", "rules_sha256", "semantic_sha256"])
    );
    assert_eq!(
        object_keys(&artifact["schema"]),
        BTreeSet::from([
            "authorization_states",
            "bytes",
            "commit",
            "definitions",
            "methods",
            "repository",
            "sha256",
            "updates",
            "version",
        ])
    );
    assert_eq!(artifact["counts"]["schema_methods"], 1_010);
    assert_eq!(artifact["counts"]["owned_methods"], 1_010);
    assert_eq!(artifact["counts"]["rules"], 17);
    assert_eq!(artifact["counts"]["overrides"], 372);
    assert_eq!(artifact["mapping_sha256"], EXPECTED_MAPPING_SHA256);
    assert_eq!(
        artifact["policy"]["semantic_sha256"],
        EXPECTED_POLICY_SEMANTIC_SHA256
    );
    assert_eq!(artifact["policy"]["rules_sha256"], EXPECTED_RULES_SHA256);
    assert_eq!(
        artifact["policy"]["overrides_sha256"],
        EXPECTED_OVERRIDES_SHA256
    );

    let methods = artifact["methods"].as_array().expect("method rows");
    assert_eq!(methods.len(), 1_010);
    assert!(methods.windows(2).all(|pair| {
        pair[0]["method"].as_str().expect("left method")
            < pair[1]["method"].as_str().expect("right method")
    }));
    let artifact_names = methods
        .iter()
        .map(|row| row["method"].as_str().expect("method name"))
        .collect::<BTreeSet<_>>();
    assert_eq!(artifact_names.len(), 1_010);
    let parsed = Schema::parse(std::str::from_utf8(&schema).expect("UTF-8 schema"))
        .expect("pinned schema parses");
    let schema_by_name = parsed
        .methods()
        .iter()
        .map(|method| (method.name(), method))
        .collect::<BTreeMap<_, _>>();
    let schema_names = schema_by_name.keys().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        artifact_names, schema_names,
        "exact method-name set equality"
    );

    let expected_row_fields = BTreeSet::from([
        "candidates",
        "canonical_signature",
        "feature_id",
        "method",
        "owner_source",
        "signature_sha256",
        "source_line",
    ]);
    let mut owner_by_name = BTreeMap::new();
    for row in methods {
        let method_name = row["method"].as_str().expect("method name");
        let definition = schema_by_name
            .get(method_name)
            .expect("artifact method exists in parsed schema");
        let feature = row["feature_id"].as_str().expect("feature ID");
        FeatureId::try_from(feature).expect("F001..F022 owner");
        assert!(owner_by_name.insert(method_name, feature).is_none());
        assert_eq!(
            row["canonical_signature"].as_str().expect("signature"),
            definition.canonical_signature(),
            "canonical signature drift for {method_name}"
        );
        assert_eq!(
            row["signature_sha256"].as_str().expect("signature hash"),
            sha256_hex(definition.canonical_signature().as_bytes()),
            "signature evidence drift for {method_name}"
        );
        assert_eq!(
            row["source_line"].as_u64().expect("source line"),
            definition.line() as u64,
            "source line drift for {method_name}"
        );
        let candidates = row["candidates"].as_array().expect("owner candidates");
        assert!(candidates.iter().any(|candidate| candidate == feature));
        let candidate_ids = candidates
            .iter()
            .map(|candidate| {
                let candidate = candidate.as_str().expect("candidate ID");
                FeatureId::try_from(candidate).expect("F001..F022 candidate");
                candidate
            })
            .collect::<Vec<_>>();
        assert!(candidate_ids.windows(2).all(|pair| pair[0] < pair[1]));
        match row["owner_source"].as_str().expect("owner source") {
            "rule" => assert_eq!(candidates.len(), 1),
            "override" => assert!(candidates.len() > 1),
            unexpected => panic!("unexpected owner source {unexpected:?}"),
        }
        assert_eq!(
            row.as_object()
                .expect("method row object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            expected_row_fields,
            "owner-only row shape"
        );
    }

    let features = artifact["features"].as_array().expect("feature summaries");
    assert_eq!(features.len(), FeatureId::ALL.len());
    for ((summary, feature_id), (expected_id, expected_count, expected_hash)) in
        features.iter().zip(FeatureId::ALL).zip(FEATURE_ORACLES)
    {
        assert_eq!(
            object_keys(summary),
            BTreeSet::from(["feature_id", "method_count", "method_set_sha256"])
        );
        assert_eq!(summary["feature_id"], feature_id.as_str());
        assert_eq!(feature_id.as_str(), expected_id);
        assert_eq!(summary["method_count"], expected_count);
        assert_eq!(summary["method_set_sha256"], expected_hash);
        let (actual_count, actual_hash) =
            feature_oracle(feature_id, &owner_by_name, &schema_by_name);
        assert_eq!(actual_count, expected_count);
        assert_eq!(actual_hash, expected_hash);
    }
    assert_eq!(
        features
            .iter()
            .map(|feature| feature["method_count"].as_u64().expect("method count"))
            .sum::<u64>(),
        1_010
    );
    assert_eq!(
        owner_assignment_sha256(&owner_by_name),
        EXPECTED_OWNER_ASSIGNMENT_SHA256,
        "exact method-to-feature assignment changed"
    );

    assert_rule_examples_agree_with_final_owners(&policy, &owner_by_name);
    assert_semantic_boundaries(&owner_by_name);
}

fn object_keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .expect("JSON object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn owner_assignment_sha256(owner_by_name: &BTreeMap<&str, &str>) -> String {
    let mut hasher = Sha256::new();
    for (method, feature) in owner_by_name {
        hasher.update(method.as_bytes());
        hasher.update([0]);
        hasher.update(feature.as_bytes());
        hasher.update(b"\n");
    }
    digest_hex(hasher.finalize())
}

fn feature_oracle(
    feature_id: FeatureId,
    owner_by_name: &BTreeMap<&str, &str>,
    schema_by_name: &BTreeMap<&str, &Definition>,
) -> (usize, String) {
    let mut count = 0;
    let mut hasher = Sha256::new();
    for (method_name, owner) in owner_by_name {
        if *owner != feature_id.as_str() {
            continue;
        }
        count += 1;
        let definition = schema_by_name
            .get(method_name)
            .expect("owned method exists in schema");
        hasher.update(method_name.as_bytes());
        hasher.update([0]);
        hasher.update(sha256_hex(definition.canonical_signature().as_bytes()).as_bytes());
        hasher.update(b"\n");
    }
    (count, digest_hex(hasher.finalize()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    digest_hex(Sha256::digest(bytes))
}

fn digest_hex(digest: impl AsRef<[u8]>) -> String {
    let mut encoded = String::with_capacity(digest.as_ref().len() * 2);
    for byte in digest.as_ref() {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn assert_rule_examples_agree_with_final_owners(
    policy_bytes: &[u8],
    owner_by_name: &BTreeMap<&str, &str>,
) {
    let policy: Value = serde_json::from_slice(policy_bytes).expect("owner policy JSON");
    for rule in policy["rules"].as_array().expect("policy rules") {
        let feature_id = rule["feature_id"].as_str().expect("rule feature ID");
        for example in rule["positive_examples"]
            .as_array()
            .expect("positive examples")
        {
            let method = example.as_str().expect("positive method example");
            assert_eq!(
                owner_by_name.get(method).copied(),
                Some(feature_id),
                "positive rule example must be finally owned by its feature: {method}"
            );
        }
    }
}

fn assert_semantic_boundaries(owner_by_name: &BTreeMap<&str, &str>) {
    let expected = [
        ("setBusinessStartPage", "F017"),
        ("getStakeDiceState", "F018"),
        ("getCallbackQueryMessage", "F012"),
        ("getCallbackQueryAnswer", "F012"),
        ("answerCallbackQuery", "F012"),
        ("answerChatJoinRequestQuery", "F012"),
        ("createCall", "F015"),
        ("acceptCall", "F015"),
        ("discardCall", "F015"),
        ("testCallEmpty", "F020"),
        ("testCallString", "F020"),
        ("testCallBytes", "F020"),
        ("testCallVectorInt", "F020"),
        ("testCallVectorIntObject", "F020"),
        ("testCallVectorString", "F020"),
        ("testCallVectorStringObject", "F020"),
        ("toggleSessionCanAcceptCalls", "F016"),
        ("toggleGroupCallAreMessagesAllowed", "F015"),
        ("getLiveStoryAvailableMessageSenders", "F015"),
        ("setLiveStoryMessageSender", "F015"),
        ("sendGroupCallMessage", "F015"),
        ("deleteGroupCallMessages", "F015"),
        ("deleteGroupCallMessagesBySender", "F015"),
        ("addMessageReaction", "F009"),
        ("getMessageEffect", "F009"),
        ("getEmojiReaction", "F014"),
        ("setStoryReaction", "F015"),
        ("setReactionNotificationSettings", "F016"),
        ("setAuthenticationEmailAddress", "F002"),
        ("checkAuthenticationEmailCode", "F002"),
        ("checkAuthenticationPassword", "F002"),
        ("checkAuthenticationPremiumPurchase", "F002"),
        ("checkAuthenticationBotToken", "F002"),
        ("acceptTermsOfService", "F002"),
        ("getPassportAuthorizationForm", "F018"),
        ("getPreferredCountryLanguage", "F018"),
        ("setApplicationVerificationToken", "F020"),
        ("setNetworkType", "F020"),
        ("getNetworkStatistics", "F019"),
        ("addNetworkStatistics", "F019"),
        ("resetNetworkStatistics", "F019"),
        ("getPremiumGiveawayPaymentOptions", "F018"),
        ("getStarGiveawayPaymentOptions", "F018"),
        ("launchPrepaidGiveaway", "F011"),
        ("getGiveawayInfo", "F011"),
        ("getTonTransactions", "F018"),
        ("getChatRevenueWithdrawalUrl", "F018"),
        ("getStarWithdrawalUrl", "F018"),
        ("getStarAdAccountUrl", "F018"),
        ("getGramWithdrawalUrl", "F018"),
        ("getChatRevenueStatistics", "F019"),
        ("getStarRevenueStatistics", "F019"),
        ("getChatRevenueTransactions", "F019"),
        ("setChatMessageAutoDeleteTime", "F009"),
        ("getDefaultMessageAutoDeleteTime", "F016"),
        ("setDefaultMessageAutoDeleteTime", "F016"),
        ("getStoryNotificationSettingsExceptions", "F016"),
        ("getTextEntities", "F009"),
        ("parseTextEntities", "F009"),
        ("parseMarkdown", "F009"),
        ("getMarkdownText", "F009"),
        ("getInternalLinkType", "F020"),
        ("getExternalLinkInfo", "F020"),
        ("getDeepLinkInfo", "F020"),
        ("getLoginUrlInfo", "F013"),
        ("getLoginUrl", "F013"),
        ("getWebAppLinkUrl", "F013"),
        ("uploadStickerFile", "F010"),
        ("createNewStickerSet", "F014"),
        ("setSupergroupStickerSet", "F011"),
        ("getPremiumInfoSticker", "F018"),
        ("getGiftChatThemes", "F020"),
        ("getUpgradedGiftEmojiStatuses", "F014"),
        ("getChatFolderInviteLinks", "F008"),
        ("createChatInviteLink", "F011"),
        ("setDefaultGroupAdministratorRights", "F012"),
        ("setDefaultChannelAdministratorRights", "F012"),
    ];

    for (method, feature) in expected {
        assert_eq!(
            owner_by_name.get(method).copied(),
            Some(feature),
            "semantic owner drift for {method}"
        );
    }
}
