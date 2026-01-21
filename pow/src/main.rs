use std::{fs::File, io::BufReader, path::PathBuf};

use clap::Parser;
use console_subscriber::ConsoleLayer;
use pow_net::Service;
use tokio::{runtime::Builder, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{Level, error, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*};

use crate::{grunt::server::GruntServer, options::{Configuration, Pipe, Protocol}};

mod options;
mod grunt;

// Use of a mod or pub mod is not actually necessary.
pub mod built_info {
   // The file has been placed there by the build script.
   include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Parser)]
#[command(version, about = "A translation layer proxy for World of Warcraft", long_about = r#"
    A translation layer proxy for World of Warcraft.

    pow ("Packet of Warcraft") is a translation layer proxy which aims to enable users to connect to modern
    classic servers using legacy clients and vice-versa. It achieves this by providing a metadata-based
    description of packet contents, seamlessly translating packets to and from different protocol versions.

    pow makes no effort to ensure the two versions can properly communicate with each other; it is the user's
    responsibility to make sure the client version they use is compatible with the server they intend to play
    on, including but not limited to:
      - Database files (models, spells, creatures, terrain...)
      - Protocol parity (1-1 translation between a protocol and another)
"#)]
struct CommandLine {
    /// A path to the configuration file for this instance of the `pow` proxy.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
}

fn open_configuration(path: Option<PathBuf>) -> anyhow::Result<Configuration> {
    let file_path = path.unwrap_or("config.json".into());
    let file = File::open(&file_path)?;
    let reader = BufReader::new(file);

    let result = serde_json::from_reader(reader)?;

    Ok(result)
}

async fn main_impl(configuration: Configuration) -> anyhow::Result<()> {
    let tasks: JoinSet<_> = configuration.pipes.into_iter()
        .map(create_pipe)
        .collect();

    // Check the return codes
    for result in tasks.join_all().await {
        match result {
            Ok(()) => (),
            Err(err) => error!("{}", err)
        };
    };

    Ok(())
}

fn rev_hash() -> String {
    match (built_info::GIT_VERSION, built_info::GIT_DIRTY) {
        (Some(v), Some(dirty)) => if dirty {
            format!("{}+", v) 
        } else {
            v.to_string()
        },
        _ => "unknown".to_string()
    }
}

fn main() -> anyhow::Result<()> {
    let logger = fmt::layer()
        .with_target(false)
        .with_timer(fmt::time::uptime())
        .with_level(true)
        .with_filter(LevelFilter::from_level(Level::INFO));
    let console_layer = ConsoleLayer::builder()
        .with_default_env()
        .spawn();

    tracing_subscriber::registry()
        .with(logger)
        .with(console_layer)
        .init();

    info!("Initializing {} {} ({}, {} build, {} {}-endian)",
        built_info::PKG_NAME,
        built_info::PKG_VERSION,
        rev_hash(),
        built_info::PROFILE,
        built_info::TARGET,
        built_info::CFG_ENDIAN);

    let command_line = CommandLine::parse();
    let configuration = match open_configuration(command_line.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("An error occured while parsing the configuration file: {}", e);
            return Ok(());
        }
    };

    Builder::new_current_thread()
        .thread_name("main")
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(main_impl(configuration))
}

async fn create_pipe(pipe: Pipe) -> anyhow::Result<()> {
    let handler = match pipe.source {
        Protocol::Grunt { host } => {
            async {
                GruntServer::new(host, CancellationToken::new())
                    .run()
                    .await
            }
        },
        Protocol::BattleNET { .. } => unimplemented!("Battle.NET servers are not implemented"),
    };

    handler.await
}
