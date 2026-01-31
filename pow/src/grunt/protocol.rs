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
use pow_packets::{Identifier, Payload, Protocol, ReadExt, Serializable, WriteExt};
use pow_macro::protocol;

use crate::grunt::protocol::{self};

#[protocol(identifier = GruntIdentifier, handlers = [
    handler(ty = LogonChallengeRequest, identifier = GruntIdentifier(0x00)),
    handler(ty = LogonProofRequest, identifier = GruntIdentifier(0x01))
])]
pub struct GruntProtocol {
    pub version: u8,
}

impl GruntProtocolImplementation for GruntProtocol {
    async fn handle_logon_challenge_request(&mut self, msg: LogonChallengeRequest) -> Result<()>  {
        todo!()
    }

    async fn handle_logon_proof_request(&mut self, msg: LogonProofRequest) -> Result<()>  {
        todo!()
    }
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
