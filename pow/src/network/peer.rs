#![allow(dead_code)]
use anyhow::Result;
use std::future::Future;

/// A remote peer is a consumer of a service that was created by an [`Acceptor`].
pub trait RemotePeer {
    fn update(&mut self) -> impl Future<Output = Result<()>>;
}


/// A local peer is a consumer of a service that was manually constructed.
/// This trait does not provide a connection primitive due to the fact that peers may need different
/// arguments when connecting to a service.
pub trait LocalPeer {
    fn disconnect(&mut self) -> impl Future<Output = Result<()>>;
}