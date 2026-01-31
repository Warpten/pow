use anyhow::Result;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::network::{Acceptor, RemotePeer, Service};
use crate::grunt::connection::GruntClient;
use crate::grunt::protocol::GruntProtocol;

/// A specialized trait for Grunt.
/// Types that implement this trait automatically implement [`Service`] and [`Acceptor`].
pub trait GruntServer {
    /// The protocol associated with this server.
    type Protocol: GruntProtocol;

    /// Returns the address on which this server binds.
    fn addr(&self) -> String;

    /// A cancellation token, that, when signaled, stops this server.
    fn token(&self) -> &CancellationToken;

    /// Creates a new protocol. This function is used by the implementation of
    /// [`Acceptor`] to create a new [`GruntClient`].
    fn make_protocol(&self) -> Self::Protocol;
}

/// Blanket implementation of [`Acceptor`] for all [`GruntServer`]s.
impl<T> Acceptor for T where T: GruntServer {
    type Peer = GruntClient<T::Protocol>;
    type Listener = TcpListener;

    fn bind(&self) -> impl Future<Output = Result<Self::Listener>> {
        async {
            Ok(TcpListener::bind(self.addr()).await?)
        }
    }

    fn next(&self, listener: &Self::Listener) -> impl Future<Output = Result<Self::Peer>> {
        async {
            let (stream, addr) = listener.accept().await?;
            let (tx, rx) = stream.into_split();

            Ok(GruntClient {
                addr,
                token: self.token().child_token(),
                sender: BufWriter::new(rx),
                reader: BufReader::new(tx),
                protocol: self.make_protocol(),
            })
        }
    }
}

/// Blanket implementation of [`Service`] for all [`GruntServer`]s.
impl<T> Service for T where T: GruntServer, T::Protocol: Send {
    type Connection = GruntClient<T::Protocol>;
    type Listener = TcpListener;

    /// Returns a cancellation token that controls the lifetime of this Grunt server.
    fn token(&self) -> &CancellationToken {
        GruntServer::token(self)
    }

    /// Runs this Grunt service and returns a future that resolves when the service is stopped.
    fn run(&self) -> impl Future<Output = Result<()>> {
        async {
            let listener = self.bind().await?;
            self.listen(listener).await
        }
    }

    /// Runs this Grunt service and returns a future that resolves when the service is stopped.
    fn listen(&self, listener: Self::Listener) -> impl Future<Output = Result<()>> {
        async move {
            info!("Grunt server listening on {}", self.addr());

            let (tx, mut rx) = mpsc::channel::<Self::Connection>(32);

            let listen_token = self.token().child_token();
            let queue_token = self.token().child_token();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        biased;

                        _ = queue_token.cancelled() => break,
                        Some(mut conn) = rx.recv() => {
                            loop {
                                match conn.update().await {
                                    Ok(_) => (),
                                    Err(err) => {
                                        error!("An error occurred while processing a packet from {}: {}", conn.addr, err);

                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            });

            loop {
                tokio::select! {
                    _ = listen_token.cancelled() => break,
                    Ok(conn) = self.next(&listener) => {
                        tx.send(conn).await?;
                    }
                }
            }

            info!("Grunt server shutting down...");
            Ok(())
        }
    }
}
