use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

use super::{
    AppErrorKind, Command, MANIFEST_PATH, OUTPUT_PATH, Outcome, POLICY_PATH, SCHEMA_PATH,
    TEMP_SUFFIX, run,
};

static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

#[test]
fn fixed_repository_check_accepts_the_committed_canonical_artifact_read_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = root.join(OUTPUT_PATH);
    let before = fs::read(&output).expect("committed owner artifact");
    let temp = output.with_file_name(format!(
        "{}{}",
        output
            .file_name()
            .expect("output file name")
            .to_string_lossy(),
        TEMP_SUFFIX
    ));
    assert!(!temp.exists());

    assert_eq!(
        run(&root, Command::Check).expect("current corpus artifact"),
        Outcome::Current
    );

    assert_eq!(fs::read(&output).expect("unchanged artifact"), before);
    assert!(!temp.exists());
}

#[test]
fn real_corpus_check_and_generate_are_canonical_bounded_and_fail_closed() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = CorpusRoot::copy_from(&repository);
    let output = corpus.root.join(OUTPUT_PATH);
    let policy = corpus.root.join(POLICY_PATH);
    let temp = publication_temp(&output);
    let canonical_output = fs::read(&output).expect("copied owner artifact");
    let canonical_policy = fs::read(&policy).expect("copied owner policy");

    assert_eq!(
        run(&corpus.root, Command::Check).expect("copied corpus is current"),
        Outcome::Current
    );
    assert_eq!(
        run(&corpus.root, Command::Generate).expect("canonical generation is unchanged"),
        Outcome::Unchanged
    );
    assert_eq!(
        fs::read(&output).expect("unchanged output"),
        canonical_output
    );
    assert!(!temp.exists());

    OpenOptions::new()
        .append(true)
        .open(&policy)
        .expect("open copied policy")
        .write_all(b" \n\t")
        .expect("append insignificant JSON whitespace");
    assert_eq!(
        run(&corpus.root, Command::Check).expect("policy whitespace is semantic-equivalent"),
        Outcome::Current
    );
    assert_eq!(
        fs::read(&output).expect("unchanged output"),
        canonical_output
    );
    assert!(!temp.exists());
    fs::write(&policy, &canonical_policy).expect("restore copied policy");

    let mut corrupted_output: Value =
        serde_json::from_slice(&canonical_output).expect("owner artifact JSON");
    corrupted_output["mapping_sha256"] = Value::String("0".repeat(64));
    fs::write(
        &output,
        serde_json::to_vec_pretty(&corrupted_output).expect("encode corrupted output"),
    )
    .expect("corrupt copied output");
    let before_check = fs::read(&output).expect("corrupted output");
    assert_eq!(
        run(&corpus.root, Command::Check)
            .expect_err("check must reject stale output")
            .kind(),
        AppErrorKind::OutOfDate
    );
    assert_eq!(fs::read(&output).expect("check is read-only"), before_check);
    assert!(!temp.exists());
    assert_eq!(
        run(&corpus.root, Command::Generate).expect("repair copied output"),
        Outcome::Updated
    );
    assert_eq!(
        fs::read(&output).expect("repaired output"),
        canonical_output
    );
    assert_eq!(
        run(&corpus.root, Command::Check).expect("repaired corpus is current"),
        Outcome::Current
    );
    assert!(!temp.exists());

    let mut corrupted_policy: Value =
        serde_json::from_slice(&canonical_policy).expect("owner policy JSON");
    corrupted_policy["overrides"][0]["signature_sha256"] = Value::String("0".repeat(64));
    fs::write(
        &policy,
        serde_json::to_vec_pretty(&corrupted_policy).expect("encode corrupted policy"),
    )
    .expect("corrupt copied policy");
    let output_before_failure = fs::read(&output).expect("output before policy failure");
    for command in [Command::Check, Command::Generate] {
        assert_eq!(
            run(&corpus.root, command)
                .expect_err("stale override evidence must fail closed")
                .kind(),
            AppErrorKind::Generation
        );
        assert_eq!(
            fs::read(&output).expect("failed command preserves output"),
            output_before_failure
        );
        assert!(
            !temp.exists(),
            "failed generation must release its temp lease"
        );
    }
}

struct CorpusRoot {
    root: PathBuf,
}

impl CorpusRoot {
    fn copy_from(repository: &Path) -> Self {
        let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "telegram-cli-owner-corpus-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create unique corpus root");
        let corpus = Self { root };
        for directory in ["vendor/tdlib", "policy", "generated"] {
            fs::create_dir_all(corpus.root.join(directory)).expect("create corpus directory");
        }
        for relative in [MANIFEST_PATH, SCHEMA_PATH, POLICY_PATH, OUTPUT_PATH] {
            fs::copy(repository.join(relative), corpus.root.join(relative))
                .expect("copy corpus input");
        }
        corpus
    }
}

impl Drop for CorpusRoot {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            if !std::thread::panicking() {
                panic!("remove owned corpus root {}: {error}", self.root.display());
            }
        }
    }
}

fn publication_temp(output: &Path) -> PathBuf {
    output.with_file_name(format!(
        "{}{}",
        output
            .file_name()
            .expect("output file name")
            .to_string_lossy(),
        TEMP_SUFFIX
    ))
}
