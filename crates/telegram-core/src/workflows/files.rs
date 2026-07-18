//! files workflows.

use super::*;

#[derive(Clone, Copy)]
pub struct DownloadQuery {
    pub file_id: i32,
    pub priority: i32,
    pub offset: i64,
    pub limit: i64,
}

pub enum InputFileSource<'source> {
    Id(i32),
    Remote(&'source str),
    Local(&'source Path),
    Generated {
        original_path: &'source Path,
        conversion: &'source str,
        expected_size: i64,
    },
}

#[derive(Clone, PartialEq, Serialize)]
pub struct FileTransferReceipt {
    pub file: Value,
    pub sequence: Option<u64>,
    pub source: TerminalSource,
    pub size_verified: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferCancellationOutcome {
    AlreadyStopped,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransferCancellationReceipt {
    pub file_id: i32,
    pub outcome: TransferCancellationOutcome,
    pub complete: bool,
}

pub fn download_file(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    query: DownloadQuery,
    deadline: Instant,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    if !(1..=32).contains(&query.priority) || query.offset < 0 || query.limit < 0 {
        return Err(ChatWorkflowError::InvalidFileTransfer);
    }
    let baseline_sequence = last_sequence(runtime);
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({
            "@type":"downloadFile",
            "file_id":query.file_id,
            "priority":query.priority,
            "offset":query.offset,
            "limit":query.limit,
            "synchronous":false
        }),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("downloadFile", response)?.into_value();
    wait_file_terminal(
        runtime,
        response,
        baseline_sequence,
        FileDirection::Download,
        deadline,
    )
}

pub fn upload_sticker_file(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    user_id: i64,
    format: StickerFormat,
    source: InputFileSource<'_>,
    deadline: Instant,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    let baseline_sequence = last_sequence(runtime);
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({
            "@type":"uploadStickerFile",
            "user_id":user_id,
            "sticker_format":format.tdjson(),
            "sticker":source.tdjson()?
        }),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("uploadStickerFile", response)?.into_value();
    wait_file_terminal(
        runtime,
        response,
        baseline_sequence,
        FileDirection::Upload,
        deadline,
    )
}

pub fn cancel_download(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    file_id: i32,
    only_if_pending: bool,
    deadline: Instant,
) -> Result<TransferCancellationReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    cancel_download_with(file_id, only_if_pending, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

#[derive(Clone, Copy)]
pub(super) enum FileDirection {
    Download,
    Upload,
}

impl InputFileSource<'_> {
    fn tdjson(&self) -> Result<Value, ChatWorkflowError> {
        Ok(match self {
            Self::Id(id) => json!({"@type":"inputFileId","id":id}),
            Self::Remote(id) if !id.is_empty() => {
                json!({"@type":"inputFileRemote","id":id})
            }
            Self::Local(file) => json!({"@type":"inputFileLocal","path":file_path(file)?}),
            Self::Generated {
                original_path,
                conversion,
                expected_size,
            } if *expected_size >= 0 => json!({
                "@type":"inputFileGenerated",
                "original_path":file_path(original_path)?,
                "conversion":conversion,
                "expected_size":expected_size
            }),
            _ => return Err(ChatWorkflowError::InvalidFileTransfer),
        })
    }
}

fn file_path(path: &Path) -> Result<&str, ChatWorkflowError> {
    path.to_str().ok_or(ChatWorkflowError::InvalidFileTransfer)
}

pub(super) fn require_uploaded_file(
    file_id: i32,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<(), ChatWorkflowError> {
    let file = call("getFile", json!({"@type":"getFile","file_id":file_id}))?;
    if file.as_value()["@type"] != "file" {
        return Err(ChatWorkflowError::UnexpectedResult { method: "getFile" });
    }
    if !file_complete(file.as_value(), FileDirection::Upload)? {
        return Err(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "uploaded_file",
        });
    }
    Ok(())
}

fn wait_file_terminal(
    runtime: &mut CoreRuntime,
    response: Value,
    baseline_sequence: u64,
    direction: FileDirection,
    deadline: Instant,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    if response["@type"] != "file" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "file transfer",
        });
    }
    let file_id = i32::try_from(required_i64(&response, "id", "file transfer")?).map_err(|_| {
        ChatWorkflowError::InvalidResult {
            method: "file transfer",
            field: "id",
        }
    })?;
    if file_complete(&response, direction)? {
        return file_receipt(response, None, TerminalSource::Response, direction);
    }
    loop {
        if let Some(file) = runtime
            .state()
            .file(file_id)
            .filter(|file| file.sequence.get() > baseline_sequence)
        {
            if file_complete(&file.value, direction)? {
                return file_receipt(
                    file.value.clone(),
                    Some(file.sequence.get()),
                    TerminalSource::OrderedUpdate,
                    direction,
                );
            }
        }
        runtime
            .next_event_until(deadline)
            .map_err(ChatWorkflowError::Runtime)?;
    }
}

fn file_complete(file: &Value, direction: FileDirection) -> Result<bool, ChatWorkflowError> {
    let (part, field) = match direction {
        FileDirection::Download => (&file["local"], "is_downloading_completed"),
        FileDirection::Upload => (&file["remote"], "is_uploading_completed"),
    };
    required_bool(part, field, "file transfer")
}

fn file_receipt(
    file: Value,
    sequence: Option<u64>,
    source: TerminalSource,
    direction: FileDirection,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    let size_verified = file_size_verified(&file, direction)?;
    Ok(FileTransferReceipt {
        file,
        sequence,
        source,
        size_verified,
        complete: true,
        observed_at: SystemTime::now(),
        freshness: match source {
            TerminalSource::Response => Freshness::ServerSnapshot,
            TerminalSource::OrderedUpdate => Freshness::OrderedUpdate,
        },
    })
}

pub(super) fn file_size_verified(
    file: &Value,
    direction: FileDirection,
) -> Result<bool, ChatWorkflowError> {
    let size = required_i64(file, "size", "file transfer")?;
    let expected = if size > 0 {
        size
    } else {
        required_i64(file, "expected_size", "file transfer")?
    };
    if expected == 0 {
        return Ok(false);
    }
    let (part, field) = match direction {
        FileDirection::Download => (&file["local"], "downloaded_size"),
        FileDirection::Upload => (&file["remote"], "uploaded_size"),
    };
    if required_i64(part, field, "file transfer")? < expected {
        return Err(ChatWorkflowError::InvalidResult {
            method: "file transfer",
            field: "terminal_size",
        });
    }
    Ok(true)
}

pub(super) fn cancel_download_with(
    file_id: i32,
    only_if_pending: bool,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<TransferCancellationReceipt, ChatWorkflowError> {
    let request = || json!({"@type":"getFile","file_id":file_id});
    let active = |file: TdObject| {
        if file.as_value()["@type"] != "file" {
            return Err(ChatWorkflowError::UnexpectedResult { method: "getFile" });
        }
        required_bool(
            &file.as_value()["local"],
            "is_downloading_active",
            "getFile",
        )
    };
    if !call("getFile", request()).and_then(active)? {
        return Ok(transfer_cancellation_receipt(
            file_id,
            TransferCancellationOutcome::AlreadyStopped,
        ));
    }
    match call(
        "cancelDownloadFile",
        json!({
            "@type":"cancelDownloadFile",
            "file_id":file_id,
            "only_if_pending":only_if_pending
        }),
    ) {
        Ok(response) => expect_ok(response, "cancelDownloadFile")?,
        Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {}
        Err(error) => return Err(error),
    }
    let outcome = match call("getFile", request()).and_then(active) {
        Ok(false) => TransferCancellationOutcome::Verified,
        Ok(true)
        | Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {
            TransferCancellationOutcome::Uncertain
        }
        Err(error) => return Err(error),
    };
    Ok(transfer_cancellation_receipt(file_id, outcome))
}

fn transfer_cancellation_receipt(
    file_id: i32,
    outcome: TransferCancellationOutcome,
) -> TransferCancellationReceipt {
    TransferCancellationReceipt {
        file_id,
        outcome,
        complete: outcome != TransferCancellationOutcome::Uncertain,
    }
}
