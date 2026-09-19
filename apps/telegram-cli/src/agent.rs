//! CLI discovery and short-lived workflow invocations; policy stays in the daemon.

use std::env;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use serde_json::{Value, json};
use telegram_protocol::{ClientErrorCode, DaemonRequest, DaemonResponse, MACHINE_PROTOCOL_VERSION};

use crate::{
    CliError, OutputFormat, acquire, exchange, parse_json, profile, response_exit, write_response,
};

pub struct Invocation {
    pub arguments: Vec<String>,
    pub format: OutputFormat,
    pub profile: String,
    pub principal: String,
    pub agent: bool,
    pub scopes: String,
}

pub fn invocation(
    mut arguments: Vec<String>,
    mut format: OutputFormat,
) -> Result<Invocation, CliError> {
    let mut profile = match env::var("TELEGRAM_PROFILE") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => "default".into(),
        Err(_) => return Err(CliError::new(ClientErrorCode::InvalidProfile)),
    };
    let mut principal = match env::var("TELEGRAM_PRINCIPAL") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => "telegram-cli".into(),
        Err(_) => return Err(CliError::new(ClientErrorCode::InvalidArguments)),
    };
    let mut scopes = "read".to_owned();
    let mut agent = false;
    let mut explicit_output = false;
    while arguments
        .first()
        .is_some_and(|arg| arg.starts_with("--") && arg != "--help" && arg != "--version")
    {
        let flag = arguments.remove(0);
        if flag == "--agent" {
            agent = true;
            continue;
        }
        if !["--profile", "--principal", "--scopes", "--output"].contains(&flag.as_str())
            || arguments.is_empty()
        {
            return Err(CliError::new(ClientErrorCode::InvalidArguments));
        }
        let value = arguments.remove(0);
        match flag.as_str() {
            "--profile" => profile = value,
            "--principal" => principal = value,
            "--scopes" => scopes = value,
            "--output" => {
                format = OutputFormat::parse(&value)?;
                explicit_output = true;
            }
            _ => unreachable!(),
        }
    }
    if !telegram_client::valid_name(&profile) {
        return Err(CliError::new(ClientErrorCode::InvalidProfile));
    }
    if agent {
        format = OutputFormat::Json;
    } else if !explicit_output {
        match env::var("TELEGRAM_OUTPUT") {
            Ok(value) => format = OutputFormat::parse(&value)?,
            Err(env::VarError::NotPresent) => {}
            Err(_) => return Err(CliError::new(ClientErrorCode::InvalidOutputFormat)),
        }
    }
    Ok(Invocation {
        arguments,
        format,
        profile,
        principal,
        agent,
        scopes,
    })
}

const HELP: &str = "Telegram CLI — одна сохранённая сессия для человека и агентов\n\
Usage: telegram-cli [--agent] [--profile NAME] [--scopes read,...] COMMAND\n\n\
  setup [--import-env]            Настроить профиль один раз и войти через owner TTY\n\
  login                          Войти (human) / прочитать состояние (JSON или --agent)\n\
  doctor                         Проверить установку без запуска TDLib и сети\n\
  init [--global] [--claude]      Установить skill (по умолчанию .agents/skills)\n\
  run WORKFLOW JSON|-            Workflow с автоматическим lease и cleanup\n\
  call JSON|-                    Raw TDLib call с автоматическим lease\n\
  workflow list|describe NAME    Найти workflow и его input_example, без аккаунта\n\
  schema version|capabilities    Закреплённая схема / ограничения API, без аккаунта\n\
  schema search TERMS            Найти методы и типы\n\
  schema describe NAME           Точная схема выбранного символа\n\
  td preview JSON                Получить exact-plan hash без выполнения\n\
  status                         Метрики активной сессии\n\n\
Продвинутый режим: session hold [SCOPES [TTL_MS]], session release LEASE,\n\
workflow run LEASE NAME JSON [APPROVAL_JSON], td call LEASE JSON [APPROVAL_JSON],\n\
events watch LEASE [CURSOR] (--output jsonl для потока).\n\
Global: --output human|json|jsonl, --principal NAME. Флаги идут до COMMAND.\n\
--agent: один JSON envelope v4, без TTY. Default scopes: read.\n\
Exit: 0 = ответ (проверяйте status: partial), 2 = input, 3 = unavailable,\n\
4 = policy/command rejected, 5 = protocol/output, 6 = cancelled.\n";

fn local_output(format: OutputFormat, data: Value, human: &str) -> Result<ExitCode, CliError> {
    let mut out = std::io::stdout().lock();
    let result = if format == OutputFormat::Human {
        writeln!(out, "{human}")
    } else {
        serde_json::to_writer(
            &mut out,
            &json!({"version": MACHINE_PROTOCOL_VERSION, "status": "ok", "data": data}),
        )
        .map_err(std::io::Error::other)
        .and_then(|_| writeln!(out))
    };
    result.map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
    Ok(ExitCode::SUCCESS)
}

pub fn local(
    args: &[String],
    format: OutputFormat,
    profile: &str,
    agent: bool,
) -> Result<Option<ExitCode>, CliError> {
    let exit = if args.is_empty() || args == ["--help"] || args == ["help"] {
        local_output(format, json!({"type": "help", "usage": HELP}), HELP)?
    } else if args == ["--version"] || args == ["version"] {
        local_output(
            format,
            json!({"type": "version", "cli_version": env!("CARGO_PKG_VERSION")}),
            concat!("telegram-cli ", env!("CARGO_PKG_VERSION")),
        )?
    } else if args == ["doctor"] {
        let data = profile::doctor(profile);
        let human = serde_json::to_string_pretty(&data).expect("doctor is serializable");
        local_output(format, data, &human)?
    } else if args.first().is_some_and(|arg| arg == "init") {
        if agent
            || args[1..]
                .iter()
                .any(|arg| arg != "--global" && arg != "--claude")
        {
            return Err(CliError::new(ClientErrorCode::InvalidArguments));
        }
        let root = if args.iter().any(|arg| arg == "--global") {
            PathBuf::from(
                env::var_os("HOME")
                    .ok_or_else(|| CliError::new(ClientErrorCode::InvalidConfiguration))?,
            )
        } else {
            env::current_dir().map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?
        };
        let root = root
            .join(if args.iter().any(|arg| arg == "--claude") {
                ".claude"
            } else {
                ".agents"
            })
            .join("skills/telegram-cli");
        std::fs::create_dir_all(root.join("agents"))
            .and_then(|_| {
                std::fs::write(
                    root.join("SKILL.md"),
                    include_str!("../../../.agents/skills/telegram-cli/SKILL.md"),
                )
            })
            .and_then(|_| {
                std::fs::write(
                    root.join("agents/openai.yaml"),
                    include_str!("../../../.agents/skills/telegram-cli/agents/openai.yaml"),
                )
            })
            .map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
        local_output(
            format,
            json!({"type": "skill_installed", "path": root}),
            "Skill telegram-cli установлен.",
        )?
    } else {
        return Ok(None);
    };
    Ok(Some(exit))
}

pub fn is_discovery(request: &DaemonRequest) -> bool {
    matches!(
        request,
        DaemonRequest::SchemaVersion
            | DaemonRequest::SchemaCapabilities
            | DaemonRequest::SchemaSearch { .. }
            | DaemonRequest::SchemaDescribe { .. }
            | DaemonRequest::WorkflowList
            | DaemonRequest::WorkflowDescribe { .. }
            | DaemonRequest::TdPreview { .. }
    )
}

pub fn discover(request: &DaemonRequest) -> Result<DaemonResponse, CliError> {
    let mut child = Command::new(profile::daemon_binary()?)
        .arg("--discover")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| CliError::new(ClientErrorCode::DaemonStartFailed))?;
    let mut input = child.stdin.take().expect("piped stdin");
    serde_json::to_writer(&mut input, request)
        .map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;
    drop(input);
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .expect("piped stdout")
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;
    let status = child
        .wait()
        .map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;
    if !status.success() || bytes.len() > 16 * 1024 * 1024 {
        return Err(CliError::new(ClientErrorCode::InvalidResponse));
    }
    serde_json::from_slice(&bytes).map_err(|_| CliError::new(ClientErrorCode::InvalidResponse))
}

pub fn automatic(
    args: &[String],
    format: OutputFormat,
    profile: &str,
    principal: String,
    scopes: &str,
) -> Result<ExitCode, CliError> {
    // Validate everything before starting a daemon or acquiring resources.
    let (workflow, input) = match args {
        [cmd, name, input] if cmd == "run" => (Some(name.clone()), parse_json(input)?),
        [cmd, input] if cmd == "call" => (None, parse_json(input)?),
        _ => return Err(CliError::new(ClientErrorCode::InvalidArguments)),
    };
    let acquire = acquire(principal.clone(), scopes, crate::DEFAULT_TTL_MS)?;
    profile::ensure_started(profile)?;
    let response = exchange(profile, &acquire)?;
    let DaemonResponse::LeaseGranted { lease } = response else {
        write_response(format, &response)
            .map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
        return Ok(response_exit(&response));
    };
    let release = DaemonRequest::LeaseRelease {
        lease_id: lease.lease_id.clone(),
        principal: principal.clone(),
    };
    let request = match workflow {
        Some(workflow) => DaemonRequest::WorkflowRun {
            lease_id: lease.lease_id,
            principal,
            workflow,
            input,
            approval: None,
        },
        None => DaemonRequest::TdCall {
            lease_id: lease.lease_id,
            principal,
            request: input,
            approval: None,
        },
    };
    // Never retry the operation after an ambiguous transport outcome. Release still runs.
    let response = exchange(profile, &request);
    let cleanup = exchange(profile, &release);
    let response = response?;
    write_response(format, &response).map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
    // Keep the operation's receipt; a failed cleanup must not invite replay of a mutation.
    if !matches!(cleanup, Ok(DaemonResponse::LeaseReleased { .. })) {
        eprintln!(
            "telegram-cli: lease cleanup pending; TTL expires automatically; do not replay the operation"
        );
    }
    Ok(response_exit(&response))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_mode_is_noninteractive_and_profile_flags_are_order_independent() {
        let args = ["--profile", "work", "--agent", "--output", "human", "login"]
            .map(str::to_owned)
            .to_vec();
        let parsed = invocation(args, OutputFormat::Human).unwrap();
        assert_eq!(parsed.format, OutputFormat::Json);
        assert!(parsed.agent);
        assert_eq!(parsed.profile, "work");
        assert!(!crate::interactive_login(&parsed.arguments, parsed.format));
        assert!(invocation(vec!["--profile".into(), "..".into()], OutputFormat::Json).is_err());
    }
}
