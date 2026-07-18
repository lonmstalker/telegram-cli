//! webapp workflows.

use super::*;

#[derive(Clone, Copy)]
pub enum WebAppMode {
    Compact,
    FullSize,
    FullScreen,
}

pub struct WebAppRequest<'request> {
    pub chat_id: i64,
    pub bot_user_id: i64,
    pub button_url: &'request str,
    pub application_name: &'request str,
    pub mode: WebAppMode,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WebAppMessageReceipt {
    pub launch_id: i64,
    pub source: Option<TerminalSource>,
    pub complete: bool,
    pub observed_at: SystemTime,
}

pub struct WebAppLease<'runtime> {
    runtime: &'runtime mut CoreRuntime,
    policy: &'runtime RawPolicy,
    launch_id: i64,
    launch_url: SensitiveString,
    require_same_origin: bool,
    baseline_sequence: u64,
    deadline: Instant,
    active: bool,
}

pub fn open_web_app<'runtime>(
    runtime: &'runtime mut CoreRuntime,
    policy: &'runtime RawPolicy,
    request: WebAppRequest<'_>,
    deadline: Instant,
) -> Result<WebAppLease<'runtime>, ChatWorkflowError> {
    require_resynced(runtime)?;
    let baseline_sequence = last_sequence(runtime);
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({
            "@type":"openWebApp",
            "chat_id":request.chat_id,
            "bot_user_id":request.bot_user_id,
            "url":request.button_url,
            "topic_id":null,
            "reply_to":null,
            "parameters":{
                "@type":"webAppOpenParameters",
                "theme":null,
                "application_name":request.application_name,
                "mode":request.mode.tdjson()
            }
        }),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("openWebApp", response)?;
    if response.as_value()["@type"] != "webAppInfo" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "openWebApp",
        });
    }
    let url = &response.as_value()["url"];
    if url["@type"] != "webAppUrl" {
        return Err(ChatWorkflowError::InvalidResult {
            method: "openWebApp",
            field: "url",
        });
    }
    Ok(WebAppLease {
        runtime,
        policy,
        launch_id: required_i64(response.as_value(), "launch_id", "openWebApp")?,
        launch_url: SensitiveString::new(required_string(url, "url", "openWebApp")?),
        require_same_origin: required_bool(url, "require_same_origin", "openWebApp")?,
        baseline_sequence,
        deadline,
        active: true,
    })
}

impl WebAppLease<'_> {
    pub fn launch_id(&self) -> i64 {
        self.launch_id
    }

    pub fn launch_url(&self) -> &SensitiveString {
        &self.launch_url
    }

    pub fn require_same_origin(&self) -> bool {
        self.require_same_origin
    }

    pub fn handoff(mut self) -> i64 {
        self.active = false;
        self.launch_id
    }

    pub fn wait_message_sent(&mut self) -> Result<WebAppMessageReceipt, ChatWorkflowError> {
        loop {
            if self
                .runtime
                .state()
                .web_app_message_sent(self.launch_id)
                .is_some_and(|sequence| sequence.get() > self.baseline_sequence)
            {
                return Ok(WebAppMessageReceipt {
                    launch_id: self.launch_id,
                    source: Some(TerminalSource::OrderedUpdate),
                    complete: true,
                    observed_at: SystemTime::now(),
                });
            }
            match self.runtime.next_event_until(self.deadline) {
                Ok(_) => {}
                Err(RuntimeError::DeadlineExceeded) => {
                    return Ok(WebAppMessageReceipt {
                        launch_id: self.launch_id,
                        source: None,
                        complete: false,
                        observed_at: SystemTime::now(),
                    });
                }
                Err(error) => return Err(ChatWorkflowError::Runtime(error)),
            }
        }
    }

    pub fn close(mut self) -> Result<(), ChatWorkflowError> {
        close_web_app_launch(self.runtime, self.policy, self.launch_id, self.deadline)?;
        self.active = false;
        Ok(())
    }
}

impl Drop for WebAppLease<'_> {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            let _ = close_web_app_launch(self.runtime, self.policy, self.launch_id, self.deadline);
        }
    }
}

impl WebAppMode {
    fn tdjson(self) -> Value {
        let mode = match self {
            Self::Compact => "webAppOpenModeCompact",
            Self::FullSize => "webAppOpenModeFullSize",
            Self::FullScreen => "webAppOpenModeFullScreen",
        };
        json!({"@type":mode})
    }
}

pub fn close_web_app_launch(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    launch_id: i64,
    deadline: Instant,
) -> Result<(), ChatWorkflowError> {
    expect_ok(
        invoke(
            runtime,
            policy,
            "closeWebApp",
            json!({"@type":"closeWebApp","web_app_launch_id":launch_id}),
            deadline,
        )?,
        "closeWebApp",
    )
}
