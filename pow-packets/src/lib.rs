mod read;
mod write;

use anyhow::Result;
pub use read::*;
pub use write::*;

/// An [`Identifier`] uniquely identifies a packet.
pub trait Identifier: Sized {
    /// The type of the protocol this identifier can encode on.
    type Protocol;

    /// This function reads the identifier from the stream.
    /// 
    /// # Arguments
    /// 
    /// - `source`: The source stream.
    /// - `protocol`: The communication [`Protocol`] in use.
    fn recv<S>(source: &mut S, protocol: &mut Self::Protocol) -> impl Future<Output = Result<Self>>
        where S: ReadExt;

    /// This function writes an identifier to the stream.
    /// 
    /// # Arguments
    /// 
    /// - `dest`: The destination stream.
    /// - `protocol`: The communication [`Protocol`] in use.
    fn send<D>(self, dest: &mut D, protocol: &mut Self::Protocol) -> impl Future<Output = Result<()>>
        where D: WriteExt;
}

/// A protocol is in charge of controlling how [`Payload`]s are (de)serialized
/// from a stream.
pub trait Protocol: Sized {
    type Identifier: Identifier<Protocol = Self>;

    /// This function:
    /// - reads an [`Identifier`] by calling [`Identifier::recv`].
    /// - switches on the value of that identifier, parses the correct [`Payload`]
    ///   and immediately handles it by calling a member method on the [`Protocol`].
    /// 
    /// The mechanism that generates the switch is yet to be decided.
    /// 
    /// # Arguments
    /// 
    /// - `source`: The stream to read from.
    fn process_incoming<S>(&mut self, source: &mut S) -> impl Future<Output = Result<()>>
        where S: ReadExt;

    /// This function:
    /// - Extracts an [`Identifier`] derived from the [`Payload`] and immediately
    ///   writes that [`Identifier`] to the stream.
    /// - Writes the [`Payload`] itself.
    /// 
    /// # Arguments
    /// 
    /// - `dest`: A stream that can be written to.
    /// - `payload`: A [`Payload`] to send.
    fn send<D, P>(&mut self, dest: &mut D, payload: P) -> impl Future<Output = Result<()>>
        where D: WriteExt, P: Payload<Protocol = Self>;
}

pub trait Serializable: Sized {
    type Protocol: Protocol;

    /// Reads this object from the given stream, using serialization parameters
    /// provided by the protocol.
    /// 
    /// # Arguments
    /// 
    /// - `source`: The source stream.
    /// - `protocol`: The communication [`Protocol`] in use.
    fn recv<S>(source: &mut S, protocol: &mut Self::Protocol) -> impl Future<Output = Result<Self>> where S: ReadExt;

    /// Sends this object on the given stream, using serialization parameters
    /// provided by the protocol.
    /// 
    /// # Arguments
    /// 
    /// - `dest`: The destination stream.
    /// - `protocol`: The communication [`Protocol`] in use.
    fn send<D>(&self, dest: &mut D, protocol: &mut Self::Protocol) -> impl Future<Output = Result<()>> where D: WriteExt;
}

/// A payload is an object that can be serialized, and that is tied to an identifier.
pub trait Payload: Sized {
    type Protocol: Protocol;

    fn identifier(&self) -> <Self::Protocol as Protocol>::Identifier;

    /// Reads this object from the given stream, using serialization parameters
    /// provided by the protocol.
    /// 
    /// # Arguments
    /// 
    /// - `source`: The source stream.
    /// - `protocol`: The communication [`Protocol`] in use.
    fn recv<S>(source: &mut S, protocol: &mut Self::Protocol) -> impl Future<Output = Result<Self>> where S: ReadExt;

    /// Sends this object on the given stream, using serialization parameters
    /// provided by the protocol.
    /// 
    /// # Arguments
    /// 
    /// - `dest`: The destination stream.
    /// - `protocol`: The communication [`Protocol`] in use.
    fn send<D>(&self, dest: &mut D, protocol: &mut Self::Protocol) -> impl Future<Output = Result<()>> where D: WriteExt;
}
