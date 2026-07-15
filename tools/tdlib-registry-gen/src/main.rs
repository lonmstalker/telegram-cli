mod app;
mod engine;

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = match app::parse_command(&args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("ошибка: {error}");
            return match error.kind() {
                app::AppErrorKind::Usage => ExitCode::from(2),
                _ => ExitCode::FAILURE,
            };
        }
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    match app::run(&root, command) {
        Ok(app::Outcome::Current) => println!("feature-owner manifest актуален"),
        Ok(app::Outcome::Updated) => println!("feature-owner manifest обновлён"),
        Ok(app::Outcome::Unchanged) => println!("feature-owner manifest уже актуален"),
        Err(error) => {
            eprintln!("ошибка: {error}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
