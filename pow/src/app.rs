use tokio::task::JoinSet;
use tracing::error;
use crate::options::{Configuration, PipeConfig, ProtocolKind};

pub async fn app(configuration: Configuration) -> anyhow::Result<()> {
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

async fn create_pipe(pipe: PipeConfig) -> anyhow::Result<()> {
    let handler = match (pipe.source, pipe.destination) {
        (ProtocolKind::Grunt { host }, ProtocolKind::BattleNET { .. }) => {},
        (ProtocolKind::BattleNET { host, port }, ProtocolKind::Grunt { .. }) => {},
        (_, _) => {
            error!("You managed to configure pow with a pipe that does nothing. That's impressive. This program will die now.");
            panic!();
        },
    };

    Ok(())
}
