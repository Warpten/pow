#![allow(unused_imports)]

mod logon_challenge;
mod logon_proof;
mod login_result;
mod realmlist;
mod security;

pub use logon_challenge::*;
pub use logon_proof::*;
pub use login_result::*;
pub use realmlist::*;
pub use security::*;

use anyhow::Result;
use pow_packets::{Identifier, Payload, Protocol, ReadExt, WriteExt};

pub struct GruntProtocol {
    pub version: u8,
}

pub struct GruntIdentifier(/* command */ u8);

impl Identifier for GruntIdentifier {
    type Protocol = GruntProtocol;

    fn recv<S>(source: &mut S, _: &mut Self::Protocol) -> impl Future<Output = Result<Self>>
        where S: ReadExt
    {
        async {
            Ok(GruntIdentifier(source.read_u8().await?))
        }
    }

    fn send<D>(self, dest: &mut D, _: &mut Self::Protocol) -> impl Future<Output = Result<()>>
        where D: WriteExt
    {
        dest.write_u8(self.0)
    }
}

impl Protocol for GruntProtocol {
    type Identifier = GruntIdentifier;
    
    fn process_incoming<S>(&mut self, source: &mut S) -> impl Future<Output = Result<()>>
        where S: ReadExt
    {
        async {
            let identifier = Self::Identifier::recv(source, self).await?;

            Ok(())
        }
    }
    
    fn send<D, P>(&mut self, dest: &mut D, payload: P) -> impl Future<Output = Result<()>>
        where D: WriteExt, P: Payload<Protocol = Self>
    {
        async move {
            payload.identifier().send(dest, self).await?;
            payload.send(dest, self).await
        }
    }
    
}