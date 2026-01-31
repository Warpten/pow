#![allow(unused_imports)]

mod logon_challenge;
mod logon_proof;
mod login_result;
mod realmlist;
mod security;

use std::io::Write;
pub use logon_challenge::*;
pub use logon_proof::*;
pub use login_result::*;
pub use realmlist::*;
pub use security::*;

use anyhow::Result;
use pow_macro::protocol;

use crate::packets::{Identifier, Payload, Protocol, ReadExt, Serializable, WriteExt};
use crate::grunt::protocol::{self};

#[protocol(identifier = GruntIdentifier, handlers = [
     handler(ty = LogonChallengeRequest, identifier = GruntIdentifier(0x00)),
     handler(ty = LogonProofRequest, identifier = GruntIdentifier(0x01))
])]
/// A Grunt-specific [`Protocol`]. Note that using this type as a constraint
/// does not imply for the given `T` to be [`Protocol`].
pub trait GruntProtocol: Send + Sync + Unpin + 'static {
    fn version(&self) -> u8;
    fn set_version(&mut self, version: u8);
}

#[derive(Debug)]
pub struct GruntIdentifier(/* command */ u8);

impl<Protocol: GruntProtocol> Identifier<Protocol> for GruntIdentifier {
    fn recv<S>(source: &mut S, _: &mut Protocol) -> impl Future<Output = Result<Self>> + Send
        where S: ReadExt
    {
        async move {
            Ok(GruntIdentifier(source.read_u8().await?))
        }
    }

    fn send<D>(self, dest: &mut D, _: &mut Protocol) -> impl Future<Output = Result<()>> + Send
        where D: WriteExt
    {
        dest.write_u8(self.0)
    }
}
