use std::net::{IpAddr, SocketAddr};
use anyhow::Result;
use pow_net::{LocalPeer, RemotePeer};
use pow_packets::{Payload, Protocol};
use tokio::{io::{AsyncWriteExt, BufReader, BufWriter}, net::{TcpStream, ToSocketAddrs, tcp::{OwnedReadHalf, OwnedWriteHalf}}};
use tokio_util::sync::CancellationToken;

use crate::grunt::protocol::GruntProtocol;

/// A [`GruntClient`] is a client able to communicate with a [`GruntServer`].
/// It is both:
/// - a [`RemotePeer`] because it can be managed by a [`GruntServer`] to model a remote.
/// - a [`LocalPeer`] because it can be created manually to connect to a [`GruntServer`].
pub struct GruntClient {
    pub(super) addr: SocketAddr,
    pub(super) token: CancellationToken,
    pub(super) sender: BufWriter<OwnedWriteHalf>,
    pub(super) reader: BufReader<OwnedReadHalf>,
    pub(super) protocol: GruntProtocol,
}

impl GruntClient {
    /// Connects to the provided server and uses the given protocol version.
    ///
    /// # Arguments
    ///
    /// - `remote`: An address to the remote server.
    /// - `protocol`: The version of the protocol to use.
    /// - `token`: A token that will close this connection once signalled.
    pub async fn connect<A: ToSocketAddrs>(remote: A, protocol: u8, token: CancellationToken) -> Result<Self> {
        let socket = TcpStream::connect(remote).await?;
        socket.set_nodelay(true)?;
        let local_address = socket.local_addr()?;

        let (tx, rx) = socket.into_split();

        Ok(Self {
            sender: BufWriter::new(rx),
            reader: BufReader::new(tx),
            protocol: GruntProtocol { version: protocol },
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
    pub async fn send<P>(&mut self, packet: P) -> Result<()>
    where
        for<'a> P: Payload<Protocol = GruntProtocol>,
    {
        packet.send(&mut self.sender, &mut self.protocol).await
    }

    /// Reads a packet from the server this client is connected to.
    pub async fn read<P>(&mut self) -> Result<P>
    where
        for<'a> P: Payload<Protocol = GruntProtocol>,
    {
        P::recv(&mut self.reader, &mut self.protocol).await
    }
}

impl RemotePeer for GruntClient {
    fn update(&mut self) -> impl Future<Output = Result<()>> {
        async {
            loop {
                tokio::select! {
                    _ = self.token.cancelled() => break,
                    _ = self.protocol.process_incoming(&mut self.reader) => ()
                }
            }

            Ok(())
        }
    }
}

impl LocalPeer for GruntClient {
    fn disconnect(&mut self) -> impl Future<Output = Result<()>> {
        async {
            self.token.cancel();
            self.sender.shutdown().await?;

            Ok(())
        }
    }
}
