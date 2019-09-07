#![forbid(unsafe_code)]

mod android;
mod init;
mod ios;
mod util;

use self::{android::AndroidCommand, init::InitCommand, ios::IosCommand};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use colored::*;
use ginit::{
    android::target::Target as AndroidTarget,
    config::Config,
    init::config_gen::interactive_config_gen,
    ios::target::Target as IosTarget,
    opts::NoiseLevel,
    target::TargetTrait as _,
    templating,
    util::{init_text_wrapper, TextWrapper},
    NAME,
};

fn cli_app<'a, 'b>(android_targets: &'a [&'a str], ios_targets: &'a [&'a str]) -> App<'a, 'b> {
    App::new(NAME)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::from_usage("-v, --verbose 'Make life louder'")
                .global(true)
                .multiple(true),
        )
        .subcommand(init::subcommand().display_order(0))
        .subcommand(android::subcommand(android_targets).display_order(1))
        .subcommand(ios::subcommand(ios_targets).display_order(1))
}

#[derive(Debug)]
struct CliInput {
    noise_level: NoiseLevel,
    command: Command,
}

impl CliInput {
    fn parse(matches: ArgMatches<'_>) -> Self {
        Self {
            noise_level: match matches.occurrences_of("verbose") {
                0 => NoiseLevel::Polite,
                1 => NoiseLevel::LoudAndProud,
                _ => NoiseLevel::FranklyQuitePedantic,
            },
            command: Command::parse(*matches.subcommand.unwrap()), // claps makes sure we got a subcommand
        }
    }
}

#[derive(Debug)]
enum Command {
    Init(InitCommand),
    Android(AndroidCommand),
    Ios(IosCommand),
}

impl Command {
    fn parse(command: SubCommand<'_>) -> Self {
        match command.name.as_str() {
            "init" => Command::Init(InitCommand::parse(command.matches)),
            "android" => Command::Android(AndroidCommand::parse(command.matches)),
            "ios" => Command::Ios(IosCommand::parse(command.matches)),
            _ => unreachable!(), // clap will reject anything else
        }
    }
}

fn get_args() -> Vec<String> {
    let mut raw: Vec<String> = std::env::args().collect();
    // Running this as a cargo subcommand gives us our name as an argument,
    // so let's just discard that...
    if raw.get(1).map(String::as_str) == Some(NAME) {
        raw.remove(1);
    }
    raw
}

fn init_log(noise_level: NoiseLevel) {
    use env_logger::{Builder, Env};
    let default_level = match noise_level {
        NoiseLevel::Polite => "warn",
        NoiseLevel::LoudAndProud => "ginit=info",
        NoiseLevel::FranklyQuitePedantic => "info",
    };
    let env = Env::default().default_filter_or(default_level);
    Builder::from_env(env).init();
}

fn handle_error(wrapper: &TextWrapper, result: Result<(), impl std::fmt::Display>) {
    if let Err(err) = result {
        eprintln!("{}", wrapper.fill(&format!("{}", err)).bright_red());
        std::process::exit(1)
    }
}

fn main() {
    let args = get_args();
    let android_targets = AndroidTarget::all()
        .keys()
        .map(|key| *key)
        .collect::<Vec<_>>();
    let ios_targets = IosTarget::all().keys().map(|key| *key).collect::<Vec<_>>();
    let app = cli_app(&android_targets, &ios_targets);
    let input = CliInput::parse(app.get_matches_from(args));
    init_log(input.noise_level);
    let wrapper = init_text_wrapper().expect("failed to init text wrapper");
    let config = Config::load(".")
        .expect("failed to load config")
        .unwrap_or_else(|| {
            let old_bike = templating::init(None);
            interactive_config_gen(&old_bike, &wrapper).expect("config gen failed");
            Config::load(".")
                .expect("failed to load config")
                .expect("no config found - did generation fail?")
        });
    match input.command {
        Command::Init(command) => handle_error(&wrapper, command.exec(&config)),
        Command::Android(command) => {
            handle_error(&wrapper, command.exec(&config, input.noise_level))
        }
        Command::Ios(command) => handle_error(&wrapper, command.exec(&config, input.noise_level)),
    }
}
