#![allow(unused)]

use futures::Stream;
use tokio_util::sync::CancellationToken;
use crate::network::Acceptor;

/// Types that implement this trait provide a (potentially endless) sequence of values through asynchronous code.
pub trait Streamable {
    type Item;

    /// Returns a stream of peers.
    fn stream(&self) -> impl Stream<Item = Self::Item>;

    /// Returns a stream of items that will end when the provided token is signalled.
    fn stream_until(&self, token: CancellationToken) -> impl Stream<Item = Self::Item>;
}

// Make any implementation of [`Acceptor`] streamable.
impl<T, P, L> Streamable for T where T: Acceptor<Peer = P, Listener = L> {
    type Item = anyhow::Result<P>;

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