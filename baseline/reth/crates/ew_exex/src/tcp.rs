//! TCP Wire Protocol Server for Execution Witnesses.

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use reth_tracing::tracing::{debug, error, info};
use alloy_primitives::BlockHash;
use reth_storage_api::BlockNumReader;

use crate::db::Database;

/// Size of the TCP frame header.
pub const HEADER_SIZE: usize = 9;
/// Message type for witnessing by block number.
pub const MSG_TYPE_EXECUTION_WITNESS_BY_BLOCK_NUMBER: u8 = 0x01;
/// Message type for witnessing by block hash.
pub const MSG_TYPE_EXECUTION_WITNESS_BY_BLOCK_HASH: u8 = 0x02;
/// Maximum payload size of 5GB.
pub const MAX_PAYLOAD_SIZE: u64 = 5 * 1024 * 1024 * 1024; // 5 GB

/// Packs a message type and payload length into a 9-byte header.
pub fn pack_header(msg_type: u8, payload_len: u64) -> [u8; HEADER_SIZE] {
    let mut header = [0u8; HEADER_SIZE];
    header[0] = msg_type;
    header[1..9].copy_from_slice(&payload_len.to_be_bytes());
    header
}

/// Unpacks a 9-byte header into a message type and payload length.
pub fn unpack_header(header: &[u8; HEADER_SIZE]) -> (u8, u64) {
    let msg_type = header[0];
    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&header[1..9]);
    let payload_len = u64::from_be_bytes(len_bytes);
    (msg_type, payload_len)
}

/// TCP service for serving indexed execution witnesses.
#[derive(Debug)]
pub struct WitnessServiceTcp<P> {
    provider: P,
    db: Arc<dyn Database>,
}

impl<P> WitnessServiceTcp<P>
where
    P: BlockNumReader + Send + Sync + 'static,
{
    /// Creates a new `WitnessServiceTcp`.
    pub const fn new(provider: P, db: Arc<dyn Database>) -> Self {
        Self { provider, db }
    }

    /// Runs the TCP server, accepting connections and serving witness payloads.
    pub async fn run_server(self: Arc<Self>, bind_addr: &str) -> eyre::Result<()> {
        let listener = TcpListener::bind(bind_addr).await?;
        info!("TCP Wire Protocol Server listening on {}", bind_addr);

        loop {
            match listener.accept().await {
                Ok((mut socket, peer_addr)) => {
                    debug!("Accepted TCP connection from {}", peer_addr);
                    let service = Arc::clone(&self);

                    tokio::spawn(async move {
                        let mut header_buf = [0u8; HEADER_SIZE];
                        if let Err(e) = socket.read_exact(&mut header_buf).await {
                            debug!("Failed to read TCP header from {}: {}", peer_addr, e);
                            return;
                        }

                        let (msg_type, payload_len) = unpack_header(&header_buf);
                        if payload_len > MAX_PAYLOAD_SIZE {
                            error!("Payload length {} exceeds MAX_PAYLOAD_SIZE of 5GB", payload_len);
                            return; // Terminate connection
                        }

                        let mut payload_buf = vec![0u8; payload_len as usize];
                        if let Err(e) = socket.read_exact(&mut payload_buf).await {
                            debug!("Failed to read TCP payload from {}: {}", peer_addr, e);
                            return;
                        }

                        let payload_opt = match msg_type {
                            MSG_TYPE_EXECUTION_WITNESS_BY_BLOCK_NUMBER => {
                                if payload_len != 8 {
                                    error!("Invalid payload length for block number request: {}", payload_len);
                                    return;
                                }
                                let number = u64::from_be_bytes(payload_buf.try_into().unwrap());
                                debug!("Received request for witness by block number: {}", number);
                                match service.db.get_raw_by_number(number).await {
                                    Ok(w) => w,
                                    Err(e) => {
                                        error!("Database error fetching witness {}: {}", number, e);
                                        None
                                    }
                                }
                            }
                            MSG_TYPE_EXECUTION_WITNESS_BY_BLOCK_HASH => {
                                if payload_len != 32 {
                                    error!("Invalid payload length for block hash request: {}", payload_len);
                                    return;
                                }
                                let hash = BlockHash::from_slice(&payload_buf);
                                debug!("Received request for witness by block hash: {}", hash);
                                match service.provider.block_number(hash) {
                                    Ok(Some(number)) => {
                                        match service.db.get_raw_by_number(number).await {
                                            Ok(w) => w,
                                            Err(e) => {
                                                error!("Database error fetching witness {}: {}", number, e);
                                                None
                                            }
                                        }
                                    }
                                    Ok(None) => None,
                                    Err(e) => {
                                        error!("Provider error resolving hash {}: {}", hash, e);
                                        None
                                    }
                                }
                            }
                            _ => {
                                error!("Unknown message type: {}", msg_type);
                                return;
                            }
                        };

                        if let Some(payload) = payload_opt {
                            let resp_header = pack_header(msg_type, payload.len() as u64);
                            if let Err(e) = socket.write_all(&resp_header).await {
                                debug!("Failed to write TCP response header to {}: {}", peer_addr, e);
                                return;
                            }
                            
                            // Stream the payload in 1MB chunks to prevent OS buffer blocking
                            let chunk_size = 1024 * 1024; // 1MB chunks
                            let mut sent = 0;
                            while sent < payload.len() {
                                let to_send = std::cmp::min(chunk_size, payload.len() - sent);
                                if let Err(e) = socket.write_all(&payload[sent..sent + to_send]).await {
                                    debug!("Failed to write TCP chunk to {}: {}", peer_addr, e);
                                    return;
                                }
                                sent += to_send;
                            }
                        } else {
                            // If witness not found, we can send back an empty payload or just close the connection.
                            let resp_header = pack_header(msg_type, 0);
                            let _ = socket.write_all(&resp_header).await;
                        }
                    });
                }
                Err(e) => {
                    error!("TCP listener accept error: {}", e);
                }
            }
        }
    }
}
