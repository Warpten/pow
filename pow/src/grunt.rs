#![allow(dead_code)]

pub mod server;
pub mod connection;
pub mod protocol;

#[cfg(test)]
mod test {
    use std::net::IpAddr;
    use std::sync::{Arc, LazyLock, RwLock};

    use pow_net::{Acceptor, LocalPeer, Service};
    use tokio_util::sync::CancellationToken;

    use crate::grunt::protocol::GruntProtocol;
    use crate::grunt::{connection::GruntClient, protocol::{LogonChallengeRequest, Version}, server::GruntServer};

    struct TestingProtocol {
        pub version: u8,
        pub recv_count: usize,
    }
    impl GruntProtocol for Arc<RwLock<TestingProtocol>> {
        fn version(&self) -> u8 {
            self.read().unwrap().version
        }

        fn set_version(&mut self, version: u8) {
            self.write().unwrap().version = version;
        }
        
        fn handle_logon_challenge_request<D>(&mut self, msg: LogonChallengeRequest, dest: &mut D) -> 
            impl ::core::future::Future<Output = anyhow::Result<()>>
            where D:crate::packets::WriteExt
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

                self.write().unwrap().recv_count += 1;

                println!("Received");
                Ok(())
            }
        }
    }

    fn make_grunt_protocol() -> Arc<RwLock<TestingProtocol>> {
        (&*PROTOCOL).clone()
    }

    static PROTOCOL: LazyLock<Arc<RwLock<TestingProtocol>>> = LazyLock::new(|| {
        Arc::new(RwLock::new(TestingProtocol {
            version: 8,
            recv_count: 0
        }))
    });

    #[tokio::test]
    #[tracing_test::traced_test]
    pub async fn test_grunt() {
        // Start the server
        let control = CancellationToken::new();
        let server = {
            let server = GruntServer::new("127.0.0.1:8080", control.child_token(), make_grunt_protocol);

            // This test doesn't use run() because we need the listener to
            // be open for the client to connect.
            let listener = server.bind().await.expect("Failed to bind");

            tokio::spawn(async move {
                server.listen(listener)
                    .await
                    .expect("Grunt server could not start listening.");
            })
        };

        // Now spin up a client, use a testing protocol, and send LOGON_CHALLENGE.
        let mut client = GruntClient::connect(
            "127.0.0.1:8080",
            make_grunt_protocol(),
            CancellationToken::new()
        ).await.expect("Unable to connect to local server");
        assert!(client.ip().is_ipv4());

        for _ in 0..10 {
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

        // TODO: Assert that the server's maintained peer's recv_count is 10.
        assert_eq!(client.protocol.read().unwrap().recv_count, 10);

        // Disconnect the client
        client.disconnect()
            .await
            .expect("Client should have disconnected");

        // Stop the server
        control.cancel();
        server.await.expect("Server should have stopped");

        
    }
}