#![allow(dead_code)]

use anyhow::Result;
use tokio::io::{AsyncWrite, AsyncWriteExt};

macro_rules! parser {
    (decl write $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<write_ $ty _be>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send;
                fn [<write_ $ty _le>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send;
            }
        )+
    };
    (impl write $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<write_ $ty _be>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send {
                    let value: $ty = value.into();
                    async move {
                        Ok(AsyncWriteExt::[<write_ $ty>](self, value).await?)
                    }
                }

                fn [<write_ $ty _le>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send {
                    let value: $ty = value.into();
                    async move {
                        Ok(AsyncWriteExt::[<write_ $ty _le>](self, value).await?)
                    }
                }
            }
        )+
    };
}

/// Provides writing utility methods on this object.
/// 
/// This trait is a Rust extension trait and is implemented for any type that implements [`AsyncWrite`] and [`Unpin`].
/// The internal implementation relies on [`AsyncWriteExt`]; this API adds methods and makes endianness explicit at the
/// call site, whereas [`AsyncWriteExt`] only makes it explicit for little-endian.
pub trait WriteExt: Send + Unpin {
    fn flush(&mut self) -> impl Future<Output = Result<()>> + Send;

    fn write_cstring<S: AsRef<str> + Send>(&mut self, str: S) -> impl Future<Output = Result<()>> + Send {
        async move {
            self.write_slice(str.as_ref().as_bytes()).await?;
            self.write_u8(0).await
        }
    }

    fn write_string<S: AsRef<str> + Send>(&mut self, str: S) -> impl Future<Output = Result<()>> + Send {
        async move {
            self.write_slice(str.as_ref().as_bytes()).await
        }
    }

    fn write_slice(&mut self, slice: &[u8]) -> impl Future<Output = Result<()>> + Send;

    fn write_u8<T: Into<u8>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send;
    fn write_i8<T: Into<i8>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send;

    parser! { decl write u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }
}

impl<Target> WriteExt for Target where Target: AsyncWrite + Unpin + Send {
    fn flush(&mut self) -> impl Future<Output = Result<()>> + Send {
        async move {
            Ok(AsyncWriteExt::flush(self).await?)
        }
    }

    fn write_slice(&mut self, slice: &[u8]) -> impl Future<Output = Result<()>> + Send {
        async move {
            Ok(AsyncWriteExt::write_all(self, slice).await?)
        }
    }

    fn write_u8<T: Into<u8>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send
        where Self: Unpin
    {
        let value: u8 = value.into();
        async move {
            Ok(AsyncWriteExt::write_u8(self, value).await?)
        }
    }

    fn write_i8<T: Into<i8>>(&mut self, value: T) -> impl Future<Output = Result<()>> + Send
        where Self: Unpin
    {
        let value: i8 = value.into();
        async move {
            Ok(AsyncWriteExt::write_i8(self, value).await?)
        }
    }

    parser! { impl write u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }
}