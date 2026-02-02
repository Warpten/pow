use anyhow::Result;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::network::{Acceptor, RemotePeer, Service};
use crate::network::connection::Client;
use crate::grunt::protocol::GruntProtocol;

/// A specialised trait for a server.
/// Types that implement this trait automatically implement [`Service`] and [`Acceptor`].
pub trait Server {
    /// The protocol associated with this server.
    type Protocol: GruntProtocol;

    /// Returns the address on which this server binds.
    fn addr(&self) -> String;

    /// A cancellation token, that, when signaled, stops this server.
    fn token(&self) -> &CancellationToken;

    /// Creates a new protocol. This function is used by the implementation of
    /// [`Acceptor`] to create a new [`Client`].
    fn make_protocol(&self) -> Self::Protocol;
}

/// Blanket implementation of [`Acceptor`] for all [`Server`]s.
impl<T> Acceptor for T where T: Server
{
    type Peer = Client<T::Protocol>;
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

            Ok(Client {
                addr,
                token: self.token().child_token(),
                sender: BufWriter::new(rx),
                reader: BufReader::new(tx),
                protocol: self.make_protocol(),
            })
        }
    }
}

/// Blanket implementation of [`Service`] for all [`Server`]s.
impl<T> Service for T where T: Server, T::Protocol: Send {
    type Connection = Client<T::Protocol>;
    type Listener = TcpListener;

    /// Returns a cancellation token that controls the lifetime of this Grunt server.
    fn token(&self) -> &CancellationToken {
        Server::token(self)
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

            Ok(())
        }
    }
}
