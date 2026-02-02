#![allow(dead_code)]

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::packets::errors::Error;

macro_rules! parser {
    (decl read $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<read_ $ty _be>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>> + Send;
                fn [<read_ $ty _le>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>> + Send;
            }
        )+
    };
    (impl read $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<read_ $ty _be>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>> + Send {
                    async move {
                        Ok(AsyncReadExt::[<read_ $ty>](self).await?.into())
                    }
                }

                fn [<read_ $ty _le>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>> + Send {
                    async move {
                        Ok(AsyncReadExt::[<read_ $ty _le>](self).await?.into())
                    }
                }
            }
        )+
    };
    (limited read $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                async fn [<read_ $ty _be>]<T: From<$ty>>(&mut self) -> Result<T> {
                    if self.limit < std::mem::size_of::<$ty>() {
                        Err(Error::EOF.into())
                    } else {
                        self.limit -= std::mem::size_of::<$ty>();
                        self.inner.[<read_ $ty _be>]().await
                    }
                }

                async fn [<read_ $ty _le>]<T: From<$ty>>(&mut self) -> Result<T> {
                    if self.limit < std::mem::size_of::<$ty>() {
                        Err(Error::EOF.into())
                    } else {
                        self.limit -= std::mem::size_of::<$ty>();
                        self.inner.[<read_ $ty _le>]().await
                    }
                }
            }
        )+
    }
}

/// Provides reading utility methods on an object.
/// 
/// This trait is a Rust extension trait and is implemented for any type that implements [`AsyncRead`] and [`Unpin`].
/// The internal implementation relies on [`AsyncReadExt`]; this API adds methods and makes endianness explicit at the
/// call site, whereas [`AsyncReadExt`] only makes it explicit for little-endian.
pub trait ReadExt: Send + Sized {
    /// Creates an adaptor which reads at most [`limit`] bytes from it.
    /// 
    /// This function returns a new instance of [`ReadExt`] which will read at most `limit` bytes, after which
    /// it will always return EOF ([`Error::EOF`]). Any read error will not count towards the number of bytes read
    /// and future calls may succeed.
    fn take<'a>(&'a mut self, limit: usize) -> Take<'a, Self>;

    /// Reads a null-terminated string from the buffer.
    /// 
    /// # Arguments
    /// 
    /// - `max_length`. If specified, this is the maximum amount of characters that will be read from the input.
    fn read_cstring(&mut self, max_length: Option<usize>) -> impl Future<Output = Result<String>> + Send {
        async move {
            let mut buf = Vec::with_capacity(max_length.unwrap_or(10));

            match max_length {
                None => loop {
                    match self.read_u8().await? {
                        0 => break,
                        value => buf.push(value)
                    };
                },
                Some(limit) => {
                    for _ in 0..limit {
                        match self.read_u8().await? {
                            0 => break,
                            value => buf.push(value)
                        };
                    }
                }
            }

            Ok(str::from_utf8(&buf[..])?.to_string())
        }
    }

    fn read_string(&mut self, length: usize) -> impl Future<Output = Result<String>> + Send {
        async move {
            let slice = self.read_slice(length).await?;

            Ok(str::from_utf8(&slice)?.to_string())
        }
    }

    fn read_slice(&mut self, size: usize) -> impl Future<Output = Result<Box<[u8]>>> + Send;
    fn read_exact_slice<const N: usize>(&mut self) -> impl Future<Output = Result<[u8; N]>> + Send;

    parser! { decl read u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }

    fn read_u8<T: From<u8>>(&mut self) -> impl Future<Output = Result<T>> + Send;
    fn read_i8<T: From<i8>>(&mut self) -> impl Future<Output = Result<T>> + Send;
}

impl<Target> ReadExt for Target where Target: AsyncRead + Unpin + Send {
    fn take<'a>(&'a mut self, limit: usize) -> Take<'a, Self> {
        Take { inner: self, limit }
    }

    fn read_u8<T: From<u8>>(&mut self) -> impl Future<Output = Result<T>> + Send {
        async {
            Ok(AsyncReadExt::read_u8(self).await?.into())
        }
    }

    fn read_i8<T: From<i8>>(&mut self) -> impl Future<Output = Result<T>> + Send {
        async {
            Ok(AsyncReadExt::read_i8(self).await?.into())
        }
    }

    fn read_slice(&mut self, size: usize) -> impl Future<Output = Result<Box<[u8]>>> + Send {
        async move {
            let mut buf = Vec::with_capacity(size);
            // SAFETY: We will read from the socket.
            unsafe { buf.set_len(size); }

            let read_count = self.read_exact(&mut buf.as_mut()).await?;
            debug_assert_eq!(read_count, size);

            Ok(buf.into_boxed_slice())
        }
    }

    fn read_exact_slice<const N: usize>(&mut self) -> impl Future<Output = Result<[u8; N]>> + Send {
        async {
            let mut buf = [0u8; N];
            
            let read_count = self.read_exact(&mut buf.as_mut()).await?;
            debug_assert_eq!(read_count, N);

            Ok(buf)
        }
    }

    parser! { impl read u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }
}

pub struct Take<'a, Inner> {
    inner: &'a mut Inner,
    limit: usize
}

impl<Inner> ReadExt for Take<'_, Inner>
    where Inner: ReadExt
{
    fn take(&mut self, limit: usize) -> Take<'_, Self> {
        Take { inner: self, limit }
    }

    fn read_u8<T: From<u8>>(&mut self) -> impl Future<Output = Result<T>> {
        async move {
            if self.limit == 0 {
                Err(Error::EOF.into())
            } else {
                let value = self.inner.read_u8().await?;
                self.limit -= 1;

                Ok(value)
            }
        }
    }

    fn read_i8<T: From<i8>>(&mut self) -> impl Future<Output = Result<T>> {
        async move {
            if self.limit == 0 {
                Err(Error::EOF.into())
            } else {
                self.limit -= 1;
                self.inner.read_i8().await
            }
        }
    }

    fn read_slice(&mut self, size: usize) -> impl Future<Output = Result<Box<[u8]>>> {
        async move {
            if self.limit < size {
                Err(Error::EOF.into())
            } else {
                self.limit -= size;
                self.inner.read_slice(size).await
            }
        }
    }

    fn read_exact_slice<const N: usize>(&mut self) -> impl Future<Output = Result<[u8; N]>> {
        async move {
            if self.limit < N {
                Err(Error::EOF.into())
            } else {
                self.limit -= N;
                self.inner.read_exact_slice().await
            }
        }
    }

    parser! { limited read u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }
}