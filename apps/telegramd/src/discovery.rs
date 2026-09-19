//! Same discovery catalogue for offline CLI and the running daemon.
use crate::server::{plan_preview, workflow_input_example};
use crate::workflow_catalog::WORKFLOWS;
use serde_json::{Value, json};
use telegram_core::raw_api::{self, SchemaDescription, SchemaSearchResult};
use telegram_core::registry::{self, SymbolKind};
use telegram_core::runtime::CoreRuntime;
use telegram_protocol::{CommandErrorCode, DaemonRequest, DaemonResponse, LeaseErrorCode};

pub fn respond(request: DaemonRequest, runtime: Option<&CoreRuntime>) -> DaemonResponse {
    match request {
        DaemonRequest::SchemaVersion => DaemonResponse::SchemaVersion {
            version: match runtime {
                Some(runtime) => serde_json::to_value(raw_api::version(runtime))
                    .expect("version is serializable"),
                None => {
                    json!({"tdlib_version": registry::SCHEMA.version, "tdlib_commit": registry::SCHEMA.commit,
                        "schema_sha256": registry::SCHEMA.sha256, "source": "pinned_schema", "runtime_verified": false})
                }
            },
        },
        DaemonRequest::SchemaCapabilities => DaemonResponse::SchemaCapabilities {
            capabilities: serde_json::to_value(raw_api::capabilities())
                .expect("capability descriptors are serializable"),
        },
        DaemonRequest::SchemaSearch { query } => DaemonResponse::SchemaSearchResults {
            results: Value::Array(
                raw_api::schema_search(&query)
                    .into_iter()
                    .map(search_result)
                    .collect(),
            ),
        },
        DaemonRequest::SchemaDescribe { name } => raw_api::schema_describe(&name).map_or_else(
            || DaemonResponse::CommandError {
                code: CommandErrorCode::SchemaNotFound,
            },
            |description| DaemonResponse::SchemaDescription {
                description: describe(description),
            },
        ),
        DaemonRequest::TdPreview { request } => match plan_preview(request) {
            Ok(preview) => DaemonResponse::TdPlanPreview { preview },
            Err(code) => DaemonResponse::CommandError { code },
        },
        DaemonRequest::WorkflowList => DaemonResponse::WorkflowList {
            workflows: WORKFLOWS
                .iter()
                .map(|entry| entry.name.to_owned())
                .collect(),
        },
        DaemonRequest::WorkflowDescribe { workflow } => match workflow_input_example(&workflow) {
            Some(input_example) => DaemonResponse::WorkflowDescription {
                workflow,
                input_example,
            },
            None => DaemonResponse::CommandError {
                code: CommandErrorCode::WorkflowNotFound,
            },
        },
        _ => DaemonResponse::Error {
            code: LeaseErrorCode::InvalidRequest,
        },
    }
}

fn search_result(result: SchemaSearchResult) -> Value {
    match result {
        SchemaSearchResult::Symbol(symbol) => json!({
            "kind": symbol_kind(symbol.kind),
            "name": symbol.name,
            "result": symbol.result.name,
        }),
        SchemaSearchResult::Type(name) => json!({"kind": "type", "name": name}),
    }
}

fn describe(description: SchemaDescription) -> Value {
    match description {
        SchemaDescription::Symbol(symbol) => {
            serde_json::to_value(symbol).expect("symbol descriptor is serializable")
        }
        SchemaDescription::Type { name, constructors } => json!({
            "kind": "type",
            "name": name,
            "constructors": constructors,
        }),
    }
}

fn symbol_kind(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Builtin => "builtin",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Method => "method",
    }
}
