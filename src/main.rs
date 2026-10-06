use clap::{Parser, Subcommand, ValueEnum};
use ntfyer::envelope::{self, Envelope};
use ntfyer::popup::macos_app::{self, Built};
use ntfyer::signal::{self, Context};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

/// Get a human's attention: popup, chime and terminal bell (macOS, Linux).
#[derive(Parser)]
#[command(name = "ntfyer", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Signal the human. Always exits 0.
    Signal {
        #[arg(long)]
        message: Option<String>,
        #[arg(long)]
        title: Option<String>,
        /// project directory (selects <project>/.ntfyer.json and the debounce key); default: cwd
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        event: Option<String>,
        /// read a JSON envelope from stdin: {message,title,project,event,session,quiet}
        #[arg(long)]
        json: bool,
    },
    /// macOS: build or refresh the notifier app. Linux: nothing to build.
    Build,
    /// Report what this machine can do.
    Doctor {
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Config helpers.
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Print the effective global and project config paths.
    Path,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

const EXIT_USAGE: u8 = 64;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            // `signal` is fail-open even on bad input: an adapter passing a flag this version does
            // not know must not break the harness that called it
            let signalling = std::env::args().nth(1).as_deref() == Some("signal");
            return match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    ExitCode::SUCCESS
                }
                _ if signalling => ExitCode::SUCCESS,
                _ => ExitCode::from(EXIT_USAGE),
            };
        }
    };
    let Some(ctx) = Context::from_env() else {
        eprintln!("ntfyer: HOME is not set");
        return match cli.cmd {
            Cmd::Signal { .. } => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        };
    };
    match cli.cmd {
        Cmd::Signal {
            message,
            title,
            project,
            event,
            json,
        } => {
            let mut env = if json {
                let mut input = String::new();
                let _ = std::io::stdin().read_to_string(&mut input);
                envelope::parse(&input)
            } else {
                Envelope::default()
            };
            env.message = message.or(env.message);
            env.title = title.or(env.title);
            env.project = project.or(env.project);
            env.event = event.or(env.event);
            signal::run(&env, &ctx, &mut std::io::stdout());
            ExitCode::SUCCESS
        }
        Cmd::Build => build(&ctx),
        Cmd::Doctor { format } => {
            let (body, healthy) = ntfyer::doctor::report(&ctx);
            match format {
                Format::Json => println!("{}", ntfyer::doctor::envelope(body)),
                Format::Text => print!("{}", ntfyer::doctor::text(&body)),
            }
            if healthy {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Cmd::Config {
            cmd: ConfigCmd::Path,
        } => {
            println!("global: {}", ctx.paths.global_config().display());
            println!(
                "project: {}",
                ntfyer::paths::project_config(&ctx.cwd).display()
            );
            ExitCode::SUCCESS
        }
    }
}

fn build(ctx: &Context) -> ExitCode {
    if ctx.os != Some(ntfyer::sound::Os::MacOs) {
        return ExitCode::SUCCESS;
    }
    let cfg = ctx.config_for(&ctx.cwd);
    match macos_app::ensure(&ctx.build_env(), &cfg.icon, true, &|p| {
        ctx.log_missing_icon(p)
    }) {
        Ok((_, Built::UpToDate)) => ExitCode::SUCCESS,
        Ok((_, Built::Fresh { compiled })) => {
            println!("{}", if compiled { "compiled" } else { "icon updated" });
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ntfyer: {e}");
            ExitCode::FAILURE
        }
    }
}
