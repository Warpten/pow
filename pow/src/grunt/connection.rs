use std::net::{IpAddr, SocketAddr};
use anyhow::Result;
use pow_net::{LocalPeer, RemotePeer};
use tokio::{io::{AsyncWriteExt, BufReader, BufWriter}, net::{TcpStream, ToSocketAddrs, tcp::{OwnedReadHalf, OwnedWriteHalf}}};
use tokio_util::sync::CancellationToken;
use tracing::{info, info_span};

use crate::{grunt::protocol::GruntProtocol, packets::{Payload, Protocol, WriteExt}};

/// A [`GruntClient`] is a client able to communicate with a [`GruntServer`].
/// It is both:
/// - a [`RemotePeer`] because it can be managed by a [`GruntServer`] to model a remote.
/// - a [`LocalPeer`] because it can be created manually to connect to a [`GruntServer`].
pub struct GruntClient<P> {
    pub(super) addr: SocketAddr,
    pub(super) token: CancellationToken,
    pub(super) sender: BufWriter<OwnedWriteHalf>,
    pub(super) reader: BufReader<OwnedReadHalf>,
    pub(super) protocol: P,
}

impl<P: GruntProtocol + Protocol> GruntClient<P> {
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

    /// Reads a packet from the server this client is connected to.
    pub async fn read<Packet>(&mut self) -> Result<Packet>
    where
        for<'a> Packet: Payload<P>,
    {
        Packet::recv(&mut self.reader, &mut self.protocol).await
    }
}

impl<P: Protocol> RemotePeer for GruntClient<P> {
    fn update(&mut self) -> impl Future<Output = Result<()>> {
        async move {
            info!("Update loop has started");

            while let Ok(_) = self.protocol.process_incoming(&mut self.reader, &mut self.sender).await {
            }

            // loop {
                
                /*tokio::select! {
                    biased;

                    _ = self.protocol.process_incoming(&mut self.reader, &mut self.sender) => (),
                    _ = self.token.cancelled() => break,
                }*/
            //}

            Ok(())
        }
    }
}

impl<P> LocalPeer for GruntClient<P> {
    fn disconnect(&mut self) -> impl Future<Output = Result<()>> {
        async {
            self.token.cancel();
            self.sender.shutdown().await?;

            Ok(())
        }
    }
}
