use anyhow::Result;
use tokio::io::{AsyncWrite, AsyncWriteExt};

macro_rules! parser {
    (decl write $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<write_ $ty _be>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>>;
                fn [<write_ $ty _le>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>>;
            }
        )+
    };
    (impl write $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<write_ $ty _be>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>> {
                    async move {
                        Ok(AsyncWriteExt::[<write_ $ty>](self, value.into()).await?)
                    }
                }

                fn [<write_ $ty _le>]<T: Into<$ty>>(&mut self, value: T) -> impl Future<Output = Result<()>> {
                    async move {
                        Ok(AsyncWriteExt::[<write_ $ty _le>](self, value.into()).await?)
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
pub trait WriteExt: Unpin {
    fn write_cstring<S: AsRef<str>>(&mut self, str: S) -> impl Future<Output = Result<()>> {
        async move {
            self.write_slice(str.as_ref().as_bytes()).await?;
            self.write_u8(0).await
        }
    }

    fn write_string<S: AsRef<str>>(&mut self, str: S) -> impl Future<Output = Result<()>> {
        async move {
            self.write_slice(str.as_ref().as_bytes()).await
        }
    }

    fn write_slice(&mut self, slice: &[u8]) -> impl Future<Output = Result<()>>;

    fn write_u8<T: Into<u8>>(&mut self, value: T) -> impl Future<Output = Result<()>>;
    fn write_i8<T: Into<i8>>(&mut self, value: T) -> impl Future<Output = Result<()>>;

    parser! { decl write u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }
}

impl<Target> WriteExt for Target where Target: AsyncWrite + Unpin {
    fn write_slice(&mut self, slice: &[u8]) -> impl Future<Output = Result<()>> {
        async {
            Ok(AsyncWriteExt::write_all(self, slice).await?)
        }
    }

    fn write_u8<T: Into<u8>>(&mut self, value: T) -> impl Future<Output = Result<()>>
        where Self: Unpin
    {
        async {
            Ok(AsyncWriteExt::write_u8(self, value.into()).await?)
        }
    }

    fn write_i8<T: Into<i8>>(&mut self, value: T) -> impl Future<Output = Result<()>>
        where Self: Unpin
    {
        async {
            Ok(AsyncWriteExt::write_i8(self, value.into()).await?)
        }
    }

    parser! { impl write u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }
}