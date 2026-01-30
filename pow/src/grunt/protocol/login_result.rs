#![allow(dead_code)]

use pow_macro::EnumKind;
use pow_packets::{ReadExt, Serializable, WriteExt};

use anyhow::Result;
use crate::grunt::protocol::GruntProtocol;


#[derive(PartialEq, PartialOrd, EnumKind, Debug)]
pub enum LoginResult {
    Success,
    UnknownFailure(u8),
    Banned,
    UnknownAccount,
    IncorrectPassword,
    AlreadyOnline,
    NoGameTime,
    DatabaseBusy,
    InvalidServer,
    DownloadFile,
    InvalidVersion,
    Suspended,
    NoAccess,
    SuccessSurvey,
    ParentalControl,
    LockedEnforced,
}

impl Serializable for LoginResult {
    type Protocol = GruntProtocol;

    async fn recv<S>(source: &mut S, protocol: &mut Self::Protocol) -> Result<Self>
        where S: ReadExt
    {
        assert!(matches!(protocol.version, 2..=3 | 5..=8));

        let value = source.read_u8().await?;

        Ok(match value {
            0x00 => LoginResult::Success,
            0x01 | 0x02 => LoginResult::UnknownFailure(value),
            0x03 => LoginResult::Banned,
            0x04 => LoginResult::UnknownAccount,
            0x05 => LoginResult::IncorrectPassword,
            0x06 => LoginResult::AlreadyOnline,
            0x07 => LoginResult::NoGameTime,
            0x08 => LoginResult::DatabaseBusy,
            0x09 => LoginResult::InvalidVersion,
            0x0A => LoginResult::DownloadFile,
            0x0B => LoginResult::InvalidServer,
            0x0C => LoginResult::Suspended,
            0x0D => LoginResult::NoAccess,
            0x0E => LoginResult::SuccessSurvey,
            0x0F => LoginResult::ParentalControl,
            0x10 if protocol.version == 8 => LoginResult::LockedEnforced,
            _ => panic!("Unknown login result {}", value)
        })
    }

    fn send<D>(&self, dest: &mut D, protocol: &mut Self::Protocol) -> impl Future<Output = Result<()>>
        where D: WriteExt
    {
        match self {
            LoginResult::Success => dest.write_u8(0x00),
            LoginResult::UnknownFailure(value) => dest.write_u8(*value),
            LoginResult::Banned => dest.write_u8(0x03),
            LoginResult::UnknownAccount => dest.write_u8(0x04),
            LoginResult::IncorrectPassword => dest.write_u8(0x05),
            LoginResult::AlreadyOnline => dest.write_u8(0x06),
            LoginResult::NoGameTime => dest.write_u8(0x07),
            LoginResult::DatabaseBusy => dest.write_u8(0x08),
            LoginResult::InvalidVersion => dest.write_u8(0x09),
            LoginResult::DownloadFile => dest.write_u8(0x0A),
            LoginResult::InvalidServer => dest.write_u8(0x0B),
            LoginResult::Suspended => dest.write_u8(0x0C),
            LoginResult::NoAccess => dest.write_u8(0x0D),
            LoginResult::SuccessSurvey => dest.write_u8(0x0E),
            LoginResult::ParentalControl => dest.write_u8(0x0F),
            LoginResult::LockedEnforced if protocol.version == 8 => dest.write_u8(0x10),
            _ => panic!("Unknown login result {:?}", self)
        }
    }
}
