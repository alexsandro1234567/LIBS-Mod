//! # Network Module
//! 
//! Network codec and compression utilities.

pub mod prediction;

use std::io::{Read, Write};

/// Compress data using zstd
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), 3)
        .map_err(|e| format!("Failed to create encoder: {}", e))?;
    
    encoder.write_all(data)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    
    encoder.finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))
}

/// Decompress data using zstd
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = zstd::stream::Decoder::new(data)
        .map_err(|e| format!("Failed to create decoder: {}", e))?;
    
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)
        .map_err(|e| format!("Failed to decompress: {}", e))?;
    
    Ok(output)
}

/// Compress data using LZ4 (faster, less compression)
pub fn compress_lz4(data: &[u8]) -> Vec<u8> {
    lz4_flex::compress_prepend_size(data)
}

/// Decompress LZ4 data
pub fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>, String> {
    lz4_flex::decompress_size_prepended(data)
        .map_err(|e| format!("LZ4 decompression failed: {}", e))
}

/// Packet header for network communication
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PacketHeader {
    /// Packet type ID
    pub packet_type: u16,
    /// Packet flags
    pub flags: u16,
    /// Payload length
    pub length: u32,
    /// Sequence number
    pub sequence: u32,
    /// Acknowledgment number
    pub ack: u32,
}

impl PacketHeader {
    /// Header size in bytes
    pub const SIZE: usize = 16;
    
    /// Create a new packet header
    pub fn new(packet_type: u16, length: u32, sequence: u32) -> Self {
        Self {
            packet_type,
            flags: 0,
            length,
            sequence,
            ack: 0,
        }
    }
    
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..2].copy_from_slice(&self.packet_type.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.flags.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.length.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.sequence.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.ack.to_le_bytes());
        bytes
    }
    
    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        
        Some(Self {
            packet_type: u16::from_le_bytes([bytes[0], bytes[1]]),
            flags: u16::from_le_bytes([bytes[2], bytes[3]]),
            length: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            sequence: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            ack: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        })
    }
}

/// Packet flags
pub mod flags {
    pub const COMPRESSED: u16 = 0x0001;
    pub const ENCRYPTED: u16 = 0x0002;
    pub const RELIABLE: u16 = 0x0004;
    pub const ORDERED: u16 = 0x0008;
    pub const FRAGMENTED: u16 = 0x0010;
}

/// Packet types
pub mod packet_type {
    pub const HANDSHAKE: u16 = 0x0001;
    pub const HEARTBEAT: u16 = 0x0002;
    pub const DISCONNECT: u16 = 0x0003;
    pub const CHUNK_DATA: u16 = 0x0010;
    pub const ENTITY_UPDATE: u16 = 0x0020;
    pub const PLAYER_INPUT: u16 = 0x0030;
    pub const WORLD_EVENT: u16 = 0x0040;
}

/// Shutdown network subsystem
pub fn shutdown() {
    log::debug!("Network subsystem shutdown");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compression_roundtrip() {
        let data = b"Hello, World! This is test data for compression.";
        
        let compressed = compress(data).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }
    
    #[test]
    fn test_lz4_roundtrip() {
        let data = b"Hello, World! This is test data for LZ4 compression.";
        
        let compressed = compress_lz4(data);
        let decompressed = decompress_lz4(&compressed).unwrap();
        
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }
    
    #[test]
    fn test_packet_header() {
        let header = PacketHeader::new(packet_type::CHUNK_DATA, 1024, 42);
        let bytes = header.to_bytes();
        let restored = PacketHeader::from_bytes(&bytes).unwrap();
        
        assert_eq!(header.packet_type, restored.packet_type);
        assert_eq!(header.length, restored.length);
        assert_eq!(header.sequence, restored.sequence);
    }
}
