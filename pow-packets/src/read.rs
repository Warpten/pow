use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt};

macro_rules! parser {
    (decl read $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<read_ $ty _be>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>>;
                fn [<read_ $ty _le>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>>;
            }
        )+
    };
    (impl read $($ty:ident),+ $(,)?) => {
        $(
            paste::paste! {
                fn [<read_ $ty _be>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>> {
                    async move {
                        Ok(AsyncReadExt::[<read_ $ty>](self).await?.into())
                    }
                }

                fn [<read_ $ty _le>]<T: From<$ty>>(&mut self) -> impl Future<Output = Result<T>> {
                    async move {
                        Ok(AsyncReadExt::[<read_ $ty _le>](self).await?.into())
                    }
                }
            }
        )+
    };
}

/// Provides reading utility methods on an object.
/// 
/// This trait is a Rust extension trait and is implemented for any type that implements [`AsyncRead`] and [`Unpin`].
/// The internal implementation relies on [`AsyncReadExt`]; this API adds methods and makes endianness explicit at the
/// call site, whereas [`AsyncReadExt`] only makes it explicit for little-endian.
pub trait ReadExt: Unpin {
    /// Reads a null-terminated string from the buffer.
    /// 
    /// # Arguments
    /// 
    /// - `max_length`. If specified, this is the maximum amount of characters that will be read from the input.
    fn read_cstring(&mut self, max_length: Option<usize>) -> impl Future<Output = Result<String>> {
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

    fn read_string(&mut self, length: usize) -> impl Future<Output = Result<String>> {
        async move {
            let slice = self.read_slice(length).await?;

            Ok(str::from_utf8(&slice)?.to_string())
        }
    }

    fn read_slice(&mut self, size: usize) -> impl Future<Output = Result<Box<[u8]>>>;
    fn read_exact_slice<const N: usize>(&mut self) -> impl Future<Output = Result<[u8; N]>>;

    parser! { decl read u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }

    fn read_u8<T: From<u8>>(&mut self) -> impl Future<Output = Result<T>>;
    fn read_i8<T: From<i8>>(&mut self) -> impl Future<Output = Result<T>>;
}

impl<Target> ReadExt for Target where Target: AsyncRead + Unpin {
    fn read_u8<T: From<u8>>(&mut self) -> impl Future<Output = Result<T>> {
        async {
            Ok(AsyncReadExt::read_u8(self).await?.into())
        }
    }

    fn read_i8<T: From<i8>>(&mut self) -> impl Future<Output = Result<T>> {
        async {
            Ok(AsyncReadExt::read_i8(self).await?.into())
        }
    }

    fn read_slice(&mut self, size: usize) -> impl Future<Output = Result<Box<[u8]>>> {
        async move {
            let mut buf = Vec::with_capacity(size);
            // SAFETY: We will read from the socket.
            unsafe { buf.set_len(size); }

            let read_count = self.read_exact(&mut buf.as_mut()).await?;
            debug_assert_eq!(read_count, size);

            Ok(buf.into_boxed_slice())
        }
    }

    fn read_exact_slice<const N: usize>(&mut self) -> impl Future<Output = Result<[u8; N]>> {
        async {
            let mut buf = [0u8; N];
            
            let read_count = self.read_exact(&mut buf.as_mut()).await?;
            debug_assert_eq!(read_count, N);

            Ok(buf)
        }
    }

    parser! { impl read u16, u32, u64, u128, i16, i32, i64, i128, f32, f64 }
}

