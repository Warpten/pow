#![allow(dead_code)]

use std::{fmt::{Debug, Display}, net::Ipv4Addr};

use anyhow::Result;
use pow_macro::EnumKind;
use pow_packets::{Payload, ReadExt, Serializable, WriteExt};

use crate::grunt::protocol::{GruntIdentifier, GruntProtocol, LoginResult, SecurityChallenge};

#[derive(Debug)]
pub struct LogonChallengeRequest {
    pub game: u32,
    pub version: Version,
    pub platform: u32,
    pub os: u32,
    pub locale: u32,
    pub timezone: i32,
    pub address: Ipv4Addr,
    pub account_name: String
}

pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub build: u16,
}

impl Version {
    pub fn parse(value: &str) -> Self {
        let mut itr = value.split('.');
        let major = itr.next()
            .map(|v| u8::from_str_radix(v, 10).ok())
            .flatten()
            .expect("major");
        let minor = itr.next()
            .map(|v| u8::from_str_radix(v, 10).ok())
            .flatten()
            .expect("minor");
        let patch = itr.next()
            .map(|v| u8::from_str_radix(v, 10).ok())
            .flatten()
            .expect("patch");
        let build = itr.next()
            .map(|v| u16::from_str_radix(v, 10).ok())
            .flatten()
            .expect("build");
        assert!(itr.next().is_none());

        Self { major, minor, patch, build }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.major, self.minor, self.patch, self.build)
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self)
    }
}

impl Payload for LogonChallengeRequest {
    type Protocol = GruntProtocol;
    type Identifier = GruntIdentifier;

    fn identifier(&self) -> GruntIdentifier {
        GruntIdentifier(0x00)
    }

    async fn recv<S>(source: &mut S, protocol: &mut Self::Protocol) -> Result<Self>
        where S: ReadExt
    {
        protocol.version = source.read_u8().await?;

        let size: usize = source.read_u8().await?;
        let mut source = source.take(size);

        let game = source.read_u32_le().await?;
        
        let version: [u8; 3] = source.read_exact_slice().await?;
        let build = source.read_u16_le().await?;

        let platform = source.read_u32_le().await?;
        let os = source.read_u32_le().await?;
        let locale = source.read_u32_le().await?;
        let timezone = source.read_i32_le().await?;
        let address = source.read_u32_be::<Ipv4Addr>().await?;
        
        let account_name = {
            let length = source.read_u8::<usize>().await?.into();
            source.read_string(length).await?
        };

        assert_eq!(size, 4 + 3 + 2 + 4 * 5 + 1 + account_name.len());

        Ok(Self {
            game,
            version : Version {
                major: version[0],
                minor: version[1],
                patch: version[2],
                build
            },
            platform,
            os,
            locale,
            timezone,
            address,
            account_name
        })
    }

    async fn send<D>(&self, dest: &mut D, protocol: &mut Self::Protocol) -> Result<()>
        where D: WriteExt
    {
        dest.write_u8(protocol.version).await?;

        let size = 4 + 1 + 1 + 1 + 2 + 4 + 4 + 4 + 4 + 4 + 1 + self.account_name.len();
        dest.write_u8(size as u8).await?;

        dest.write_u32_le(self.game).await?;
        dest.write_u8(self.version.major).await?;
        dest.write_u8(self.version.minor).await?;
        dest.write_u8(self.version.patch).await?;
        dest.write_u16_le(self.version.build).await?;
        dest.write_u32_le(self.platform).await?;
        dest.write_u32_le(self.os).await?;
        dest.write_u32_le(self.locale).await?;
        dest.write_i32_le(self.timezone).await?;
        dest.write_slice(&self.address.octets()).await?;

        dest.write_u8(self.account_name.len() as u8).await?;
        dest.write_slice(self.account_name.as_bytes()).await?;

        Ok(())
    }
}

#[derive(EnumKind, Debug)]
pub enum LogonChallengeResponse {
    Ok {
        public_key: [u8; 32],
        generator: Box<[u8]>,
        large_safe_prime: Box<[u8]>,
        salt: [u8; 32],
        crc: [u8; 16],
        security: SecurityChallenge,
    },
    Err(LoginResult)
}

impl Payload for LogonChallengeResponse {
    type Protocol = GruntProtocol;
    type Identifier = GruntIdentifier;
    
    fn identifier(&self) -> Self::Identifier {
        GruntIdentifier(0x00)
    }

    async fn recv<S>(source: &mut S, protocol: &mut Self::Protocol) -> Result<Self>
        where S: ReadExt
    {
        let login_result = LoginResult::recv(source, protocol).await?;
        if login_result == LoginResult::Success {
            let public_key = source.read_exact_slice().await?;
            let generator = {
                let size = source.read_u8::<usize>().await?;
                source.read_slice(size).await?
            };

            let large_safe_prime = {
                let size = source.read_u8::<usize>().await?;
                source.read_slice(size).await?
            };

            let salt = source.read_exact_slice().await?;
            let crc = source.read_exact_slice().await?;

            let security = SecurityChallenge::recv(source, protocol).await?;

            Ok(Self::Ok {
                public_key,
                generator,
                large_safe_prime,
                salt,
                crc,
                security
            })
        } else {
            Ok(Self::Err(login_result))
        }
    }

    async fn send<D>(&self, dest: &mut D, protocol: &mut Self::Protocol) -> Result<()>
        where D: WriteExt
    {
        dest.write_u8(0).await?; // Most emulators write a zero here.

        match self {
            LogonChallengeResponse::Ok { public_key, generator, large_safe_prime, salt, crc, security } => {
                dest.write_u8(0).await?; // LoginResult::Success
                dest.write_slice(public_key).await?;
                
                dest.write_u8(generator.len() as u8).await?;
                dest.write_slice(&generator).await?;

                dest.write_u8(large_safe_prime.len() as u8).await?;
                dest.write_slice(&large_safe_prime).await?;

                dest.write_slice(salt).await?;
                dest.write_slice(crc).await?;
                
                security.send(dest, protocol).await
            },
            LogonChallengeResponse::Err(login_result) => {
                dest.write_u8(login_result.identifier() as u8).await
            },
        }
    }
}