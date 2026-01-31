#![allow(dead_code)]

pub mod server;
pub mod connection;
pub mod protocol;

#[cfg(test)]
mod test {
    use std::net::IpAddr;
    use tokio::sync::broadcast::{self, Receiver, Sender};
    use tokio_util::sync::CancellationToken;
    use anyhow::Result;
    use tracing::info;
    use crate::grunt::protocol::{GruntProtocol, LogonProofRequest};
    use crate::grunt::{connection::GruntClient, protocol::{LogonChallengeRequest, Version}, server::GruntServer};
    use crate::network::{LocalPeer, Acceptor, Service};
    use crate::packets::WriteExt;

    const PACKET_COUNT: usize = 10;
    const SERVER_ADDRESS: &'static str = "127.0.0.1:8080";

    /// This type is a very simple server for which [`GruntServer`] will be implemented.
    /// It holds:
    /// - The [`CancellationToken`] that controls its lifetime.
    /// - A [`broadcast::Sender`] that will be cloned whenever a new [`GruntClient`] connects.
    ///   Each client will have an associated [`ServerProtocol`] that will use this sender to
    ///   signal that a packet was successfully received.
    struct Server {
        sender: Sender<u32>,
        token: CancellationToken,
    }

    impl GruntServer for Server {
        type Protocol = ServerProtocol;

        fn addr(&self) -> String { SERVER_ADDRESS.to_string() }

        fn token(&self) -> &CancellationToken {
            &self.token
        }

        fn make_protocol(&self) -> Self::Protocol {
            ServerProtocol {
                version: 8,
                signal: self.sender.clone(),
            }
        }
    }

    /// A test utility method that simply waits for signals that packets were received.
    async fn all_requests_handled(mut receiver: Receiver<u32>) {
        for i in 0..PACKET_COUNT {
            match receiver.recv().await {
                Ok(_) => info!("Received packet {} of {}", i + 1, PACKET_COUNT),
                Err(e) => assert!(false, "Failed to receive a packet: {}", e)
            };
        }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    pub async fn test_round_trip() {
        let controller = CancellationToken::new();
        let (tx, rx) = broadcast::channel(10);

        // Spin up the server in another thread.
        let server = {
            let server = Server {
                sender: tx,
                token: controller.child_token(),
            };

            // This test doesn't use run() because we need the listener to
            // be open for the client to connect. The call to bind() needs
            // to happen now so that the client will connect on an open
            // socket.
            let listener = server.bind().await.expect("Failed to bind");

            tokio::spawn(async move {
                server.listen(listener)
                    .await
                    .expect("Grunt server could not start listening.");
            })
        };

        // Now spin up a client. We use a clone of the protocol the server
        // uses just so we can share state between the client and the server
        // for this test.
        let mut client = GruntClient::connect(
            SERVER_ADDRESS,
            ClientProtocol { version: 8 },
            CancellationToken::new()
        ).await.expect("Unable to connect to local server");
        assert!(client.ip().is_ipv4());

        // Span a task that will complete when all packets have been processed.
        let test = tokio::spawn(all_requests_handled(rx));

        // Spin up a task that will send packets.
        let send_task = tokio::spawn(async move {
            for _ in 0..PACKET_COUNT {
                match client.ip() {
                    IpAddr::V4(addr) => client.send(LogonChallengeRequest {
                        game: 0x00576F57, // WoW\0
                        version: Version::parse("4.3.4.15595"),
                        platform: 0x00783836, // x86\0
                        os: 0x4F5358, // OSX\0
                        locale: 0x656E5553, // enUS
                        timezone: 0x3C,
                        address: addr,
                        account_name: "pow".to_string()
                    }).await.expect("Packet couldn't be sent"),
                    IpAddr::V6(..) => panic!("Not an ipv4 address")
                }
            }

            // Disconnect the client
            client.disconnect()
                .await
                .expect("Client should have disconnected");
        });

        // Block until both tasks are complete.
        let _ = tokio::join! { send_task, test };

        // Stop the server - this should pretty much instantly yield.
        controller.cancel();
        server.await.expect("Server should have stopped");
    }

    /// A simple protocol for Grunt. This is more or less exactly the same type as the
    /// [`TestingProtocol`] but it lacks the signal state and will panic if it suddenly
    /// starts behaving as a [`GruntServer`].
    struct ClientProtocol {
        pub version: u8
    }

    impl GruntProtocol for ClientProtocol {
        fn version(&self) -> u8 { self.version }
        fn set_version(&mut self, version: u8) {
            self.version = version;
        }

        async fn handle_logon_challenge_request<D>(&mut self, _: LogonChallengeRequest, _: &mut D)
            -> Result<()>
                where D: WriteExt
        {
            Ok(assert!(false, "Should never be called"))
        }
        
        async fn handle_logon_proof_request<D>(&mut self, _: LogonProofRequest, _: &mut D)
            -> Result<()>
                where D:crate::packets::WriteExt
        {
            Ok(assert!(false, "Should never be called"))
        }
    }

    /// This is the protocol that is associated with the [`TestingServer`].
    struct ServerProtocol {
        pub version: u8,
        pub signal: Sender<u32>
    }

    impl GruntProtocol for ServerProtocol {
        fn version(&self) -> u8 {
            self.version
        }

        fn set_version(&mut self, version: u8) {
            self.version = version;
        }

        fn handle_logon_challenge_request<D>(&mut self, msg: LogonChallengeRequest, _: &mut D)
            -> impl Future<Output = Result<()>>  where D: WriteExt
        {
            async move {
                assert_eq!(msg.game, 0x00576F57);
                assert_eq!(msg.version.major, 4, "Invalid version");
                assert_eq!(msg.version.minor, 3, "Invalid version");
                assert_eq!(msg.version.patch, 4, "Invalid version");
                assert_eq!(msg.version.build, 15595, "Invalid version");
                assert_eq!(msg.platform, 0x00783836);
                assert_eq!(msg.os, 0x4F5358);
                assert_eq!(msg.locale, 0x656E5553);
                assert_eq!(msg.account_name, "pow");

                self.signal.send(1)
                    .expect("Unable to signal");

                Ok(())
            }
        }
        
        // This test does not send this packet.
        async fn handle_logon_proof_request<D>(&mut self, _: LogonProofRequest, _: &mut D)
            -> Result<()>
                where D:crate::packets::WriteExt
        {
            Ok(assert!(false, "Should never be called"))
        }
    }
}