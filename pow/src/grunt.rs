#![allow(dead_code)]

pub mod server;
pub mod connection;
pub mod protocol;

#[cfg(test)]
mod test {
    use std::net::IpAddr;

    use pow_net::{Acceptor, LocalPeer, Service};
    use tokio_util::sync::CancellationToken;

    use crate::grunt::{connection::GruntClient, protocol::{LogonChallengeRequest, Version}, server::GruntServer};

    #[tokio::test]
    #[tracing_test::traced_test]
    pub async fn test_grunt() {
        // Start the server
        let control = CancellationToken::new();
        let server = {
            let server = GruntServer::new("127.0.0.1:8080", control.child_token());

            // This test doesn't use run() because we need the listener to
            // be open for the client to connect.
            let listener = server.bind().await.expect("Failed to bind");

            tokio::spawn(async move {
                server.listen(listener)
                    .await
                    .expect("Grunt server could not start listening.");
            })
        };
        
        // Now spin up a client and send LOGON_CHALLENGE.
        let mut client = GruntClient::connect("127.0.0.1:8080", 8, CancellationToken::new())
            .await
            .expect("Unable to connect to local server");
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

        // TODO: Assert that the server receives correct data.
        
        // Disconnect the client
        client.disconnect()
            .await
            .expect("Client should have disconnected");

        // Stop the server
        control.cancel();
        server.await.expect("Server should have stopped");
    }
}