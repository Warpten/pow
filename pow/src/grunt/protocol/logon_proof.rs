#![allow(dead_code)]

use pow_packets::{ReadExt, Serializable, WriteExt};

use anyhow::Result;
use crate::grunt::protocol::{GruntProtocol, SecurityProof};

#[derive(Debug)]
pub struct TelemetryKey {
    pub unk1: u16,
    pub unk2: u32,
    pub unk3: [u8; 4],
    pub proof: [u8; 20],
}

impl Serializable for TelemetryKey {
    type Protocol = GruntProtocol;

    async fn recv<S>(source: &mut S, _: &mut Self::Protocol) -> Result<Self>
        where S: ReadExt
    {
        let unk1 = source.read_u16_le().await?;
        let unk2 = source.read_u32_le().await?;
        let unk3 = source.read_exact_slice().await?;
        let proof = source.read_exact_slice().await?;

        Ok(Self { unk1, unk2, unk3, proof })
    }
    
    async fn send<D>(&self, dest: &mut D, _: &mut Self::Protocol) -> Result<()>
        where D: WriteExt
    {
        dest.write_u16_le(self.unk1).await?;
        dest.write_u32_le(self.unk2).await?;
        dest.write_slice(&self.unk3).await?;
        dest.write_slice(&self.proof).await
    }
}

#[derive(Debug)]
pub struct LogonProofRequest {
    pub public_key: [u8; 32],
    pub proof: [u8; 20],
    pub crc: [u8; 20],
    pub telemetry_keys: Vec<TelemetryKey>,
    pub security: SecurityProof,
}

impl Serializable for LogonProofRequest {
    type Protocol = GruntProtocol;

    async fn recv<S>(source: &mut S, protocol: &mut Self::Protocol) -> Result<Self>
        where S: ReadExt
    {
        let public_key = source.read_exact_slice().await?;
        let proof = source.read_exact_slice().await?;
        let crc = source.read_exact_slice().await?;

        let telemetry_keys = {
            let length = source.read_u8().await?;
            let mut keys = Vec::with_capacity(length as usize);
            for _ in 0..length {
                keys.push(TelemetryKey::recv(source, protocol).await?);
            }
            keys
        };

        let security = SecurityProof::recv(source, protocol).await?;

        Ok(Self {
            public_key,
            proof,
            crc,
            telemetry_keys,
            security,
        })
    }

    async fn send<D>(&self, dest: &mut D, protocol: &mut Self::Protocol) -> Result<()>
        where D: WriteExt
    {
        dest.write_slice(&self.public_key).await?;
        dest.write_slice(&self.proof).await?;
        dest.write_slice(&self.crc).await?;
        dest.write_u8(self.telemetry_keys.len() as u8).await?;
        for telemetry_key in &self.telemetry_keys {
            telemetry_key.send(dest, protocol).await?;
        }

        self.security.send(dest, protocol).await
    }
}

#[derive(Debug)]
pub struct LogonProofResponse {

}
