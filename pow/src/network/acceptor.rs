#![allow(dead_code)]
use crate::network::RemotePeer;

/// An acceptor listens on a resource and passively initializes a service.
pub trait Acceptor: Sized {
    /// The type of object this acceptor provides. This models a client.
    type Peer: RemotePeer;

    /// The listener to use.
    type Listener;

    /// Binds this acceptor and returns the listener it is bound to.
    fn bind(&self) -> impl Future<Output = anyhow::Result<Self::Listener>>;

    /// Returns a future that will resolve when a [`Peer`] connects.
    fn next(&self, listener: &Self::Listener) -> impl Future<Output = anyhow::Result<Self::Peer>>;
}