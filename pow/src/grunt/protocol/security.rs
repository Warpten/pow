#![allow(dead_code)]

use pow_macro::EnumKind;

use anyhow::Result;
use crate::packets::{ReadExt, Serializable, WriteExt};
use crate::grunt::protocol::GruntProtocol;

#[derive(PartialEq, EnumKind, Debug)]
pub enum SecurityChallenge {
    None,
    Pin { seed: u32, salt: [u8; 16] },
    Matrix { width: u8, height: u8, digits: u8, challenges: u8, seed: u64 },
    Authenticator(u8)
}

impl<P: GruntProtocol> Serializable<P> for SecurityChallenge {
    async fn recv<S>(source: &mut S, protocol: &mut P) -> Result<Self>
        where S: ReadExt
    {
        if protocol.version() != 2 {
            let kind = source.read_u8().await?;
            Ok(match kind {
                0 => Self::None,
                1 => {
                    let seed = source.read_u32_le().await?;
                    let salt = source.read_exact_slice().await?;

                    Self::Pin { seed, salt }
                },
                2 => {
                    let width = source.read_u8().await?;
                    let height = source.read_u8().await?;
                    let digits = source.read_u8().await?;
                    let count = source.read_u8().await?;
                    let seed = source.read_u64_le().await?;

                    Self::Matrix { width, height, digits, challenges: count, seed }
                },
                3 => {
                    Self::Authenticator(source.read_u8().await?)
                },
                _ => panic!("Unknown security kind")
            })
        } else {
            Ok(Self::None)
        }
    }

    async fn send<D>(self, dest: &mut D, protocol: &mut P) -> Result<()>
        where D: WriteExt
    {
        if protocol.version() == 2 {
            return Ok(());
        }

        let kind = self.identifier() as u8;
        dest.write_u8(kind).await?;

        Ok(match self {
            SecurityChallenge::None => (),
            SecurityChallenge::Pin { seed, salt } => {
                dest.write_u32_le(seed).await?;
                dest.write_slice(&salt).await?;
            },
            SecurityChallenge::Matrix { width, height, digits, challenges, seed } => {
                dest.write_u8(width).await?;
                dest.write_u8(height).await?;
                dest.write_u8(digits).await?;
                dest.write_u8(challenges).await?;
                dest.write_u64_le(seed).await?;
            },
            SecurityChallenge::Authenticator(value) => {
                dest.write_u8(value).await?;
            },
        })
    }
}

#[derive(PartialEq, EnumKind, Debug)]
pub enum SecurityProof {
    None,
    Pin { salt: [u8; 16], hash: [u8; 16] },
    Matrix { proof: [u8; 20] },
    Authenticator(String)
}

impl<P: GruntProtocol> Serializable<P> for SecurityProof {
    async fn recv<S>(source: &mut S, protocol: &mut P) -> Result<Self>
        where S: ReadExt
    {
        if protocol.version() != 2 {
            let kind = source.read_u8().await?;
            Ok(match kind {
                0 => Self::None,
                1 => {
                    let salt = source.read_exact_slice().await?;
                    let hash = source.read_exact_slice().await?;

                    Self::Pin { salt, hash }
                },
                2 => {
                    let proof = source.read_exact_slice().await?;

                    Self::Matrix { proof }
                },
                3 => {
                    let length = source.read_u8::<usize>().await?;
                    let str = source.read_string(length).await?;
                    Self::Authenticator(str)
                },
                _ => panic!("Unknown security kind")
            })
        } else {
            Ok(Self::None)
        }
    }

    async fn send<D>(self, dest: &mut D, protocol: &mut P) -> Result<()>
        where D: WriteExt
    {
        if protocol.version() == 2 {
            assert!(self == Self::None);
            return Ok(());
        }

        let kind = self.identifier() as u8;
        dest.write_u8(kind).await?;

        Ok(match self {
            SecurityProof::None => (),
            SecurityProof::Pin { salt, hash } => {
                dest.write_slice(&salt).await?;
                dest.write_slice(&hash).await?;
            },
            SecurityProof::Matrix { proof } => {
                dest.write_slice(&proof).await?;
            },
            SecurityProof::Authenticator(str) => {
                dest.write_u8(str.len() as u8).await?;
                dest.write_slice(str.as_bytes()).await?;
            },
        })
    }
}