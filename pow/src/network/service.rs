#![allow(unused)]
use tokio_util::sync::CancellationToken;

/// A [`Service`] manages multiple [`RemotePeer`]s. A [`Service`] is generally
/// an [`Accessor`], but the relation is not equivalent.
pub trait Service {
    /// The type of connection this server opens for a client. This type is a [`RemotePer`].
    type Connection;

    /// The type of listener this server uses to accept [`RemotePeer`]s.
    type Listener;

    /// Returns a cancellation token that controls the lifetime of this service.
    fn token(&self) -> &CancellationToken;

    /// Runs this service and returns a future that will resolve when the service stops.
    fn run(&self) -> impl Future<Output = anyhow::Result<()>>;

    /// Runs this service and returns a future that will resolve when the service stops.
    ///
    /// # Arguments
    ///
    /// - `listener`: The network listener to listen on.
    fn listen(&self, listener: Self::Listener) -> impl Future<Output = anyhow::Result<()>>;
}