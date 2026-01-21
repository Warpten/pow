use pow_net::{Acceptor, RemotePeer, Service};
use anyhow::Result;
use tokio::{io::{BufReader, BufWriter}, net::TcpListener, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::grunt::{protocol::GruntProtocol, connection::GruntClient};

/// A [`GruntServer`] is a service that accepts connections from clients and communicates with them using the Grunt protocol.
/// It is therefore a [`Server`] and an [`Acceptor`].
pub struct GruntServer {
    addr: String,
    token: CancellationToken,
}

impl GruntServer {
    pub fn new<A>(bind: A, token: CancellationToken) -> Self
        where A : Into<String>
    {
        Self {
            addr: bind.into(),
            token,
        }
    }
}

impl Acceptor for GruntServer {
    type Peer = GruntClient;
    type Listener = TcpListener;

    fn bind(&self) -> impl Future<Output = Result<Self::Listener>> {
        async {
            Ok(TcpListener::bind(&self.addr).await?)
        }
    }
    
    fn next(&self, listener: &Self::Listener) -> impl Future<Output = Result<Self::Peer>> {
        async {
            let (stream, addr) = listener.accept().await?;
            let (tx, rx) = stream.into_split();

            Ok(GruntClient {
                addr,
                token: self.token.child_token(),
                sender: BufWriter::new(rx),
                reader: BufReader::new(tx),
                protocol: GruntProtocol { version: 0 }
            })
        }
    }
}

impl Service for GruntServer {
    type Connection = GruntClient;
    type Listener = TcpListener;

    fn token(&self) -> &CancellationToken {
        &self.token
    }

    fn listen(&self, listener: Self::Listener) -> impl Future<Output = Result<()>> {
        async move {
            let (tx, mut rx) = mpsc::channel::<Self::Connection>(32);
            let queue_token = self.token().child_token();

            let worker_task = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = queue_token.cancelled() => {
                            return;
                        },
                        Some(mut conn) = rx.recv() => {
                            tokio::spawn(async move {
                                // There's a token on the connection itself... that was derived from
                                // this server's token. It'll stop update() appropriately.
                                while let Err(e) = conn.update().await {
                                    error!("{}: {}", conn.ip(), e);
                                }
                            });
                        }
                    }
                }
            });

            loop {
                tokio::select! {
                    _ = self.token().cancelled() => break,
                    Ok(conn) = self.next(&listener) => {
                        tx.send(conn).await?;
                    }
                }
            }

            let _ = worker_task.await;
            Ok(())
        }
    }
    
    fn run(&self) -> impl Future<Output = Result<()>> {
        async {
            let listener = self.bind().await?;
            self.listen(listener).await
        }
    }
}

