use anyhow::Result;
use futures::{Stream};
use tokio_util::sync::CancellationToken;

/// Types that implement this trait provide a (potentially endless) sequence of values through asynchronous code.
pub trait Streamable {
    type Item;

    /// Returns a stream of peers.
    fn stream(&self) -> impl Stream<Item = Self::Item>;

    /// Returns a stream of items that will end when the provided token is signalled.
    fn stream_until(&self, token: CancellationToken) -> impl Stream<Item = Self::Item>;
}

/// An acceptor listens on a resource and passively initializes a service.
pub trait Acceptor: Sized {
    /// The type of object this acceptor provides. This models a client.
    type Peer: RemotePeer;

    /// The listener to use.
    type Listener;

    /// Binds this acceptor and returns the listener it is bound to.
    fn bind(&self) -> impl Future<Output = Result<Self::Listener>>;

    /// Returns a future that will resolve when a client connects.
    fn next(&self, listener: &Self::Listener) -> impl Future<Output = Result<Self::Peer>>;
}

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

/// A [`Service`] manages multiple [`RemotePeer`]s. A [`Service`] is generally
/// an [`Accessor`], but the relation is not equivalent.
pub trait Service {
    type Connection;
    type Listener;

    /// Returns a cancellation token that controls the lifetime of this service.
    fn token(&self) -> &CancellationToken;

    /// Runs this service and returns a future that will resolve when the service stops.
    fn run(&self) -> impl Future<Output = Result<()>>;

    /// Runs this service and returns a future that will resolve when the service stops.
    fn listen(&self, listener: Self::Listener) -> impl Future<Output = Result<()>>;
}

// Make any implementation of [`Acceptor`] streamable.
impl<T, P, L> Streamable for T where T: Acceptor<Peer = P, Listener = L> {
    type Item = Result<P>;

    fn stream(&self) -> impl Stream<Item = Self::Item> {
        async_stream::try_stream! {
            let listener = self.bind().await?;
            loop { 
                yield self.next(&listener).await?;
            }
        }
    }

    fn stream_until(&self, token: CancellationToken) -> impl Stream<Item = Self::Item> {
        let bind_token = token.child_token();

        async_stream::try_stream! {
            let listener = self.bind().await?;
            loop {
                tokio::select! {
                    _ = bind_token.cancelled() => break,
                    Ok(conn) = self.next(&listener) => yield conn,
                    else => break
                }
            }
        }
    }
}

