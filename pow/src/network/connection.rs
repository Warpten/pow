use std::net::{IpAddr, SocketAddr};
use anyhow::Result;
use crate::network::LocalPeer;
use tokio::{io::{AsyncWriteExt, BufReader, BufWriter}, net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream, ToSocketAddrs}};
use tokio_util::sync::CancellationToken;
use crate::network::RemotePeer;
use crate::{grunt::protocol::GruntProtocol, packets::{Payload, Protocol}};

/// A [`Client`] is a client able to communicate with a [`Server`].
/// It is both:
/// - a [`RemotePeer`] because it can be managed by a [`Server`] to model a remote.
/// - a [`LocalPeer`] because it can be created manually to connect to a [`Server`].
pub struct Client<P> {
    pub(crate) addr: SocketAddr,
    pub(crate) token: CancellationToken,
    pub(crate) sender: BufWriter<OwnedWriteHalf>,
    pub(crate) reader: BufReader<OwnedReadHalf>,
    pub(crate) protocol: P,
}

impl<P: GruntProtocol + Protocol> Client<P> {
    /// Connects to the provided server and uses the given protocol version.
    ///
    /// # Arguments
    ///
    /// - `remote`: An address to the remote server.
    /// - `protocol`: The version of the protocol to use.
    /// - `token`: A token that will close this connection once signalled.
    pub async fn connect<A: ToSocketAddrs>(remote: A, protocol: P, token: CancellationToken) -> Result<Self> {
        let socket = TcpStream::connect(remote).await?;
        socket.set_nodelay(true)?;
        let local_address = socket.local_addr()?;

        let (tx, rx) = socket.into_split();

        Ok(Self {
            sender: BufWriter::new(rx),
            reader: BufReader::new(tx),
            protocol,
            addr: local_address,
            token,
        })
    }

    pub fn ip(&self) -> IpAddr {
        self.addr.ip()
    }

    /// Sends the given packet to the server this client is connected to.
    ///
    /// # Arguments
    ///
    /// - `packet`: The packet to send.
    pub fn send<Packet>(&mut self, packet: Packet) -> impl Future<Output = Result<()>>
    where
        for<'a> Packet: Payload<P>,
    {
        self.protocol.send(&mut self.sender, packet)
    }
}

impl<P: Protocol> RemotePeer for Client<P> {
    fn update(&mut self) -> impl Future<Output = Result<()>>  {
        async move {
            loop {
                tokio::select! {
                    _ = self.token.cancelled() => break,
                    _ = self.protocol.process_incoming(&mut self.reader, &mut self.sender) => (),
                };
            }

            Ok(())
        }
    }
}

impl<P> LocalPeer for Client<P> {
    fn disconnect(&mut self) -> impl Future<Output = Result<()>> {
        async {
            self.token.cancel();
            self.sender.shutdown().await?;

            Ok(())
        }
    }
}
