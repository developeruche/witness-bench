use ethrex_blockchain::error::ChainError;
use ethrex_blockchain::payload::PayloadBuildResult;
use ethrex_common::types::block_execution_witness::{ExecutionWitness, RpcExecutionWitness};
use ethrex_common::types::payload::PayloadBundle;
use ethrex_common::types::requests::{EncodedRequests, compute_requests_hash};
use ethrex_common::types::{Block, BlockBody, BlockHash, BlockNumber, Fork};
use ethrex_common::{H256, U256};
use ethrex_p2p::sync::SyncMode;
use ethrex_rlp::error::RLPDecodeError;
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

use crate::rpc::{RpcApiContext, RpcHandler};
use crate::types::payload::{
    ExecutionPayload, ExecutionPayloadBody, ExecutionPayloadBodyV2, ExecutionPayloadResponse,
    PayloadStatus,
};
use crate::utils::RpcErr;
use crate::utils::{RpcRequest, parse_json_hex};

// Must support rquest sizes of at least 32 blocks
// Chosen an arbitrary x4 value
// -> https://github.com/ethereum/execution-apis/blob/main/src/engine/shanghai.md#specification-3
const GET_PAYLOAD_BODIES_REQUEST_MAX_SIZE: u64 = 128;

// NewPayload V1-V2-V3 implementations
pub struct NewPayloadV1Request {
    pub payload: ExecutionPayload,
}

impl RpcHandler for NewPayloadV1Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(NewPayloadV1Request {
            payload: parse_execution_payload(params)?,
        })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        validate_execution_payload_v1(&self.payload)?;
        let block = match get_block_from_payload(&self.payload, None, None, None) {
            Ok(block) => block,
            Err(err) => {
                return Ok(serde_json::to_value(PayloadStatus::invalid_with_err(
                    &err.to_string(),
                ))?);
            }
        };
        let payload_status = handle_new_payload_v1_v2(&self.payload, block, context).await?;
        serde_json::to_value(payload_status).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct NewPayloadV2Request {
    pub payload: ExecutionPayload,
}

impl RpcHandler for NewPayloadV2Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(NewPayloadV2Request {
            payload: parse_execution_payload(params)?,
        })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let chain_config = &context.storage.get_chain_config();
        if chain_config.is_shanghai_activated(self.payload.timestamp) {
            validate_execution_payload_v2(&self.payload)?;
        } else {
            // Behave as a v1
            validate_execution_payload_v1(&self.payload)?;
        }
        let block = match get_block_from_payload(&self.payload, None, None, None) {
            Ok(block) => block,
            Err(err) => {
                return Ok(serde_json::to_value(PayloadStatus::invalid_with_err(
                    &err.to_string(),
                ))?);
            }
        };
        let payload_status = handle_new_payload_v1_v2(&self.payload, block, context).await?;
        serde_json::to_value(payload_status).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct NewPayloadV3Request {
    pub payload: ExecutionPayload,
    pub expected_blob_versioned_hashes: Vec<H256>,
    pub parent_beacon_block_root: H256,
}

impl From<NewPayloadV3Request> for RpcRequest {
    fn from(val: NewPayloadV3Request) -> Self {
        RpcRequest {
            method: "engine_newPayloadV3".to_string(),
            params: Some(vec![
                serde_json::json!(val.payload),
                serde_json::json!(val.expected_blob_versioned_hashes),
                serde_json::json!(val.parent_beacon_block_root),
            ]),
            ..Default::default()
        }
    }
}

impl RpcHandler for NewPayloadV3Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 3 {
            return Err(RpcErr::BadParams("Expected 3 params".to_owned()));
        }
        Ok(NewPayloadV3Request {
            payload: serde_json::from_value(params[0].clone())
                .map_err(|_| RpcErr::WrongParam("payload".to_string()))?,
            expected_blob_versioned_hashes: serde_json::from_value(params[1].clone())
                .map_err(|_| RpcErr::WrongParam("expected_blob_versioned_hashes".to_string()))?,
            parent_beacon_block_root: serde_json::from_value(params[2].clone())
                .map_err(|_| RpcErr::WrongParam("parent_beacon_block_root".to_string()))?,
        })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let block = match get_block_from_payload(
            &self.payload,
            Some(self.parent_beacon_block_root),
            None,
            None,
        ) {
            Ok(block) => block,
            Err(err) => {
                return Ok(serde_json::to_value(PayloadStatus::invalid_with_err(
                    &err.to_string(),
                ))?);
            }
        };
        validate_fork(&block, Fork::Cancun, &context)?;
        validate_execution_payload_v3(&self.payload)?;
        let payload_status = handle_new_payload_v3(
            &self.payload,
            context,
            block,
            self.expected_blob_versioned_hashes.clone(),
        )
        .await?;
        serde_json::to_value(payload_status).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct NewPayloadV4Request {
    pub payload: ExecutionPayload,
    pub expected_blob_versioned_hashes: Vec<H256>,
    pub parent_beacon_block_root: H256,
    pub execution_requests: Vec<EncodedRequests>,
}

impl From<NewPayloadV4Request> for RpcRequest {
    fn from(val: NewPayloadV4Request) -> Self {
        RpcRequest {
            method: "engine_newPayloadV4".to_string(),
            params: Some(vec![
                serde_json::json!(val.payload),
                serde_json::json!(val.expected_blob_versioned_hashes),
                serde_json::json!(val.parent_beacon_block_root),
                serde_json::json!(val.execution_requests),
            ]),
            ..Default::default()
        }
    }
}

impl RpcHandler for NewPayloadV4Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 4 {
            return Err(RpcErr::BadParams("Expected 4 params".to_owned()));
        }
        Ok(NewPayloadV4Request {
            payload: serde_json::from_value(params[0].clone())
                .map_err(|_| RpcErr::WrongParam("payload".to_string()))?,
            expected_blob_versioned_hashes: serde_json::from_value(params[1].clone())
                .map_err(|_| RpcErr::WrongParam("expected_blob_versioned_hashes".to_string()))?,
            parent_beacon_block_root: serde_json::from_value(params[2].clone())
                .map_err(|_| RpcErr::WrongParam("parent_beacon_block_root".to_string()))?,
            execution_requests: serde_json::from_value(params[3].clone())
                .map_err(|_| RpcErr::WrongParam("execution_requests".to_string()))?,
        })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        // validate the received requests
        validate_execution_requests(&self.execution_requests)?;

        let requests_hash = compute_requests_hash(&self.execution_requests);
        let block = match get_block_from_payload(
            &self.payload,
            Some(self.parent_beacon_block_root),
            Some(requests_hash),
            None,
        ) {
            Ok(block) => block,
            Err(err) => {
                return Ok(serde_json::to_value(PayloadStatus::invalid_with_err(
                    &err.to_string(),
                ))?);
            }
        };

        let chain_config = context.storage.get_chain_config();

        if !chain_config.is_prague_activated(block.header.timestamp) {
            return Err(RpcErr::UnsuportedFork(format!(
                "{:?}",
                chain_config.get_fork(block.header.timestamp)
            )));
        }
        // We use v3 since the execution payload remains the same.
        validate_execution_payload_v3(&self.payload)?;
        let payload_status = handle_new_payload_v3(
            &self.payload,
            context,
            block,
            self.expected_blob_versioned_hashes.clone(),
        )
        .await?;
        serde_json::to_value(payload_status).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct NewPayloadV5Request {
    pub payload: ExecutionPayload,
    pub expected_blob_versioned_hashes: Vec<H256>,
    pub parent_beacon_block_root: H256,
    pub execution_requests: Vec<EncodedRequests>,
}

impl From<NewPayloadV5Request> for RpcRequest {
    fn from(val: NewPayloadV5Request) -> Self {
        RpcRequest {
            method: "engine_newPayloadV5".to_string(),
            params: Some(vec![
                serde_json::json!(val.payload),
                serde_json::json!(val.expected_blob_versioned_hashes),
                serde_json::json!(val.parent_beacon_block_root),
                serde_json::json!(val.execution_requests),
            ]),
            ..Default::default()
        }
    }
}

impl RpcHandler for NewPayloadV5Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 4 {
            return Err(RpcErr::BadParams("Expected 4 params".to_owned()));
        }
        Ok(Self {
            payload: serde_json::from_value(params[0].clone())
                .map_err(|_| RpcErr::WrongParam("payload".to_string()))?,
            expected_blob_versioned_hashes: serde_json::from_value(params[1].clone())
                .map_err(|_| RpcErr::WrongParam("expected_blob_versioned_hashes".to_string()))?,
            parent_beacon_block_root: serde_json::from_value(params[2].clone())
                .map_err(|_| RpcErr::WrongParam("parent_beacon_block_root".to_string()))?,
            execution_requests: serde_json::from_value(params[3].clone())
                .map_err(|_| RpcErr::WrongParam("execution_requests".to_string()))?,
        })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        validate_execution_payload_v4(&self.payload)?;

        // validate the received requests
        validate_execution_requests(&self.execution_requests)?;

        let requests_hash = compute_requests_hash(&self.execution_requests);
        let block_access_list_hash = self
            .payload
            .block_access_list
            .as_ref()
            .map(|b| b.compute_hash());

        let block = match get_block_from_payload(
            &self.payload,
            Some(self.parent_beacon_block_root),
            Some(requests_hash),
            block_access_list_hash,
        ) {
            Ok(block) => block,
            Err(err) => {
                return Ok(serde_json::to_value(PayloadStatus::invalid_with_err(
                    &err.to_string(),
                ))?);
            }
        };

        let chain_config = context.storage.get_chain_config();

        if !chain_config.is_amsterdam_activated(block.header.timestamp) {
            return Err(RpcErr::UnsuportedFork(format!(
                "{:?}",
                chain_config.get_fork(block.header.timestamp)
            )));
        }
        let payload_status = handle_new_payload_v4(
            &self.payload,
            context,
            block,
            self.expected_blob_versioned_hashes.clone(),
        )
        .await?;
        serde_json::to_value(payload_status).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

// GetPayload V1-V2-V3 implementations
pub struct GetPayloadV1Request {
    pub payload_id: u64,
}

impl RpcHandler for GetPayloadV1Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let payload_id = parse_get_payload_request(params)?;
        Ok(Self { payload_id })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let payload_bundle = get_payload(self.payload_id, &context).await?;
        // NOTE: This validation is actually not required to run Hive tests. Not sure if it's
        // necessary
        validate_payload_v1_v2(&payload_bundle.block, &context)?;

        // V1 doesn't support BAL (pre-EIP-7928)
        let response = ExecutionPayload::from_block(payload_bundle.block, None);

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct GetPayloadV2Request {
    pub payload_id: u64,
}

impl RpcHandler for GetPayloadV2Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let payload_id = parse_get_payload_request(params)?;
        Ok(Self { payload_id })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let payload_bundle = get_payload(self.payload_id, &context).await?;
        validate_payload_v1_v2(&payload_bundle.block, &context)?;

        // V2 doesn't support BAL (pre-EIP-7928)
        let response = ExecutionPayloadResponse {
            execution_payload: ExecutionPayload::from_block(payload_bundle.block, None),
            block_value: payload_bundle.block_value,
            blobs_bundle: None,
            should_override_builder: None,
            execution_requests: None,
        };

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct GetPayloadV3Request {
    pub payload_id: u64,
}

impl From<GetPayloadV3Request> for RpcRequest {
    fn from(val: GetPayloadV3Request) -> Self {
        RpcRequest {
            method: "engine_getPayloadV3".to_string(),
            params: Some(vec![serde_json::json!(U256::from(val.payload_id))]),
            ..Default::default()
        }
    }
}

impl RpcHandler for GetPayloadV3Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let payload_id = parse_get_payload_request(params)?;
        Ok(Self { payload_id })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let payload_bundle = get_payload(self.payload_id, &context).await?;
        validate_fork(&payload_bundle.block, Fork::Cancun, &context)?;

        // V3 doesn't support BAL (Cancun fork, pre-EIP-7928)
        let response = ExecutionPayloadResponse {
            execution_payload: ExecutionPayload::from_block(payload_bundle.block, None),
            block_value: payload_bundle.block_value,
            blobs_bundle: Some(payload_bundle.blobs_bundle),
            should_override_builder: Some(false),
            execution_requests: None,
        };

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct GetPayloadV4Request {
    pub payload_id: u64,
}

impl From<GetPayloadV4Request> for RpcRequest {
    fn from(val: GetPayloadV4Request) -> Self {
        RpcRequest {
            method: "engine_getPayloadV4".to_string(),
            params: Some(vec![serde_json::json!(U256::from(val.payload_id))]),
            ..Default::default()
        }
    }
}

impl RpcHandler for GetPayloadV4Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let payload_id = parse_get_payload_request(params)?;
        Ok(Self { payload_id })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let payload_bundle = get_payload(self.payload_id, &context).await?;
        let chain_config = &context.storage.get_chain_config();

        if !chain_config.is_prague_activated(payload_bundle.block.header.timestamp) {
            return Err(RpcErr::UnsuportedFork(format!(
                "{:?}",
                chain_config.get_fork(payload_bundle.block.header.timestamp)
            )));
        }
        if chain_config.is_osaka_activated(payload_bundle.block.header.timestamp) {
            return Err(RpcErr::UnsuportedFork(format!("{:?}", Fork::Osaka)));
        }

        // V4 doesn't support BAL (Prague fork, pre-EIP-7928)
        let response = ExecutionPayloadResponse {
            execution_payload: ExecutionPayload::from_block(payload_bundle.block, None),
            block_value: payload_bundle.block_value,
            blobs_bundle: Some(payload_bundle.blobs_bundle),
            should_override_builder: Some(false),
            execution_requests: Some(
                payload_bundle
                    .requests
                    .into_iter()
                    .filter(|r| !r.is_empty())
                    .collect(),
            ),
        };

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct GetPayloadV5Request {
    pub payload_id: u64,
}

impl From<GetPayloadV5Request> for RpcRequest {
    fn from(val: GetPayloadV5Request) -> Self {
        RpcRequest {
            method: "engine_getPayloadV5".to_string(),
            params: Some(vec![serde_json::json!(U256::from(val.payload_id))]),
            ..Default::default()
        }
    }
}

impl RpcHandler for GetPayloadV5Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let payload_id = parse_get_payload_request(params)?;
        Ok(Self { payload_id })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let payload_bundle = get_payload(self.payload_id, &context).await?;
        let chain_config = &context.storage.get_chain_config();

        if !chain_config.is_osaka_activated(payload_bundle.block.header.timestamp) {
            return Err(RpcErr::UnsuportedFork(format!(
                "{:?}",
                chain_config.get_fork(payload_bundle.block.header.timestamp)
            )));
        }

        // V5 supports BAL (Amsterdam fork, EIP-7928)
        let response = ExecutionPayloadResponse {
            execution_payload: ExecutionPayload::from_block(
                payload_bundle.block,
                payload_bundle.block_access_list,
            ),
            block_value: payload_bundle.block_value,
            blobs_bundle: Some(payload_bundle.blobs_bundle),
            should_override_builder: Some(false),
            execution_requests: Some(
                payload_bundle
                    .requests
                    .into_iter()
                    .filter(|r| !r.is_empty())
                    .collect(),
            ),
        };

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

pub struct GetPayloadBodiesByHashV1Request {
    pub hashes: Vec<BlockHash>,
}

impl RpcHandler for GetPayloadBodiesByHashV1Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 1 {
            return Err(RpcErr::BadParams("Expected 1 param".to_owned()));
        };

        Ok(GetPayloadBodiesByHashV1Request {
            hashes: serde_json::from_value(params[0].clone())?,
        })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        if self.hashes.len() as u64 >= GET_PAYLOAD_BODIES_REQUEST_MAX_SIZE {
            return Err(RpcErr::TooLargeRequest);
        }
        let mut bodies = Vec::new();
        for hash in self.hashes.iter() {
            bodies.push(context.storage.get_block_body_by_hash(*hash).await?)
        }
        build_payload_body_response(bodies)
    }
}

pub struct GetPayloadBodiesByRangeV1Request {
    start: BlockNumber,
    count: u64,
}

impl RpcHandler for GetPayloadBodiesByRangeV1Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams("Expected 1 param".to_owned()));
        };
        let start = parse_json_hex(&params[0]).map_err(|_| RpcErr::BadHexFormat(0))?;
        let count = parse_json_hex(&params[1]).map_err(|_| RpcErr::BadHexFormat(1))?;
        if start < 1 {
            return Err(RpcErr::WrongParam("start".to_owned()));
        }
        if count < 1 {
            return Err(RpcErr::WrongParam("count".to_owned()));
        }
        Ok(GetPayloadBodiesByRangeV1Request { start, count })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        if self.count >= GET_PAYLOAD_BODIES_REQUEST_MAX_SIZE {
            return Err(RpcErr::TooLargeRequest);
        }
        let latest_block_number = context.storage.get_latest_block_number().await?;
        // NOTE: we truncate the range because the spec says we "MUST NOT return trailing
        // null values if the request extends past the current latest known block"
        let last = latest_block_number.min(self.start + self.count - 1);
        let bodies = context.storage.get_block_bodies(self.start, last).await?;
        build_payload_body_response(bodies)
    }
}

fn build_payload_body_response(bodies: Vec<Option<BlockBody>>) -> Result<Value, RpcErr> {
    let response: Vec<Option<ExecutionPayloadBody>> = bodies
        .into_iter()
        .map(|body| body.map(Into::into))
        .collect();
    serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
}

// ==================== V2 Body Methods (EIP-7928) ====================

pub struct GetPayloadBodiesByHashV2Request {
    pub hashes: Vec<BlockHash>,
}

impl RpcHandler for GetPayloadBodiesByHashV2Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 1 {
            return Err(RpcErr::BadParams("Expected 1 param".to_owned()));
        };

        Ok(GetPayloadBodiesByHashV2Request {
            hashes: serde_json::from_value(params[0].clone())?,
        })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        if self.hashes.len() as u64 >= GET_PAYLOAD_BODIES_REQUEST_MAX_SIZE {
            return Err(RpcErr::TooLargeRequest);
        }

        let mut bodies: Vec<Option<ExecutionPayloadBodyV2>> = Vec::new();
        for hash in &self.hashes {
            let block = context.storage.get_block_by_hash(*hash).await?;
            let result = match block {
                Some(block) => {
                    let bal = context
                        .blockchain
                        .generate_bal_for_block(&block)
                        .map_err(|e| RpcErr::Internal(e.to_string()))?;
                    Some(ExecutionPayloadBodyV2::from_body_with_bal(block.body, bal))
                }
                None => None,
            };
            bodies.push(result);
        }

        serde_json::to_value(bodies).map_err(|e| RpcErr::Internal(e.to_string()))
    }
}

pub struct GetPayloadBodiesByRangeV2Request {
    start: BlockNumber,
    count: u64,
}

impl RpcHandler for GetPayloadBodiesByRangeV2Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams("Expected 2 params".to_owned()));
        };
        let start = parse_json_hex(&params[0]).map_err(|_| RpcErr::BadHexFormat(0))?;
        let count = parse_json_hex(&params[1]).map_err(|_| RpcErr::BadHexFormat(1))?;
        if start < 1 {
            return Err(RpcErr::WrongParam("start".to_owned()));
        }
        if count < 1 {
            return Err(RpcErr::WrongParam("count".to_owned()));
        }
        Ok(GetPayloadBodiesByRangeV2Request { start, count })
    }

    async fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        if self.count >= GET_PAYLOAD_BODIES_REQUEST_MAX_SIZE {
            return Err(RpcErr::TooLargeRequest);
        }
        let latest_block_number = context.storage.get_latest_block_number().await?;
        // NOTE: we truncate the range because the spec says we "MUST NOT return trailing
        // null values if the request extends past the current latest known block"
        let last = latest_block_number.min(self.start + self.count - 1);

        // Bulk fetch bodies (like V1)
        let block_bodies = context.storage.get_block_bodies(self.start, last).await?;

        let mut bodies: Vec<Option<ExecutionPayloadBodyV2>> = Vec::new();
        for (i, body_opt) in block_bodies.into_iter().enumerate() {
            let block_number = self.start + i as u64;
            let result = match body_opt {
                Some(body) => {
                    // Get header for this block
                    let header =
                        context
                            .storage
                            .get_block_header(block_number)?
                            .ok_or_else(|| {
                                RpcErr::Internal(format!(
                                    "Header not found for block {block_number}"
                                ))
                            })?;
                    let block = Block { header, body };

                    let bal = context
                        .blockchain
                        .generate_bal_for_block(&block)
                        .map_err(|e| RpcErr::Internal(e.to_string()))?;
                    Some(ExecutionPayloadBodyV2::from_body_with_bal(block.body, bal))
                }
                None => None,
            };
            bodies.push(result);
        }

        serde_json::to_value(bodies).map_err(|e| RpcErr::Internal(e.to_string()))
    }
}

fn parse_execution_payload(params: &Option<Vec<Value>>) -> Result<ExecutionPayload, RpcErr> {
    let params = params
        .as_ref()
        .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
    if params.len() != 1 {
        return Err(RpcErr::BadParams("Expected 1 param".to_owned()));
    }
    serde_json::from_value(params[0].clone()).map_err(|_| RpcErr::WrongParam("payload".to_string()))
}

fn validate_execution_payload_v1(payload: &ExecutionPayload) -> Result<(), RpcErr> {
    // Validate that only the required arguments are present
    if payload.withdrawals.is_some() {
        return Err(RpcErr::WrongParam("withdrawals".to_string()));
    }
    if payload.blob_gas_used.is_some() {
        return Err(RpcErr::WrongParam("blob_gas_used".to_string()));
    }
    if payload.excess_blob_gas.is_some() {
        return Err(RpcErr::WrongParam("excess_blob_gas".to_string()));
    }

    Ok(())
}

fn validate_execution_payload_v2(payload: &ExecutionPayload) -> Result<(), RpcErr> {
    // Validate that only the required arguments are present
    if payload.withdrawals.is_none() {
        return Err(RpcErr::WrongParam("withdrawals".to_string()));
    }
    if payload.blob_gas_used.is_some() {
        return Err(RpcErr::WrongParam("blob_gas_used".to_string()));
    }
    if payload.excess_blob_gas.is_some() {
        return Err(RpcErr::WrongParam("excess_blob_gas".to_string()));
    }

    Ok(())
}

fn validate_execution_payload_v3(payload: &ExecutionPayload) -> Result<(), RpcErr> {
    // Validate that only the required arguments are present
    if payload.withdrawals.is_none() {
        return Err(RpcErr::WrongParam("withdrawals".to_string()));
    }
    if payload.blob_gas_used.is_none() {
        return Err(RpcErr::WrongParam("blob_gas_used".to_string()));
    }
    if payload.excess_blob_gas.is_none() {
        return Err(RpcErr::WrongParam("excess_blob_gas".to_string()));
    }

    Ok(())
}

#[inline]
fn validate_execution_payload_v4(payload: &ExecutionPayload) -> Result<(), RpcErr> {
    // This method follows the same specification as `engine_newPayloadV4` additionally
    // rejects payload without block access list

    if payload.block_access_list.is_none() {
        return Err(RpcErr::WrongParam("block_access_list".to_string()));
    }

    validate_execution_payload_v3(payload)?;

    Ok(())
}

fn validate_payload_v1_v2(block: &Block, context: &RpcApiContext) -> Result<(), RpcErr> {
    let chain_config = &context.storage.get_chain_config();
    if chain_config.is_cancun_activated(block.header.timestamp) {
        return Err(RpcErr::UnsuportedFork(
            "Cancun payload received".to_string(),
        ));
    }
    Ok(())
}

// This function is used to make sure neither the current block nor its parent have been invalidated
async fn validate_ancestors(
    block: &Block,
    context: &RpcApiContext,
) -> Result<Option<PayloadStatus>, RpcErr> {
    // Check if the block has already been invalidated
    if let Some(latest_valid_hash) = context
        .storage
        .get_latest_valid_ancestor(block.hash())
        .await?
    {
        return Ok(Some(PayloadStatus::invalid_with(
            latest_valid_hash,
            "Header has been previously invalidated.".into(),
        )));
    }

    // Check if the parent block has already been invalidated
    if let Some(latest_valid_hash) = context
        .storage
        .get_latest_valid_ancestor(block.header.parent_hash)
        .await?
    {
        // Invalidate child too
        context
            .storage
            .set_latest_valid_ancestor(block.header.hash(), latest_valid_hash)
            .await?;
        return Ok(Some(PayloadStatus::invalid_with(
            latest_valid_hash,
            "Parent header has been previously invalidated.".into(),
        )));
    }

    Ok(None)
}

async fn handle_new_payload_v1_v2(
    payload: &ExecutionPayload,
    block: Block,
    context: RpcApiContext,
) -> Result<PayloadStatus, RpcErr> {
    let Some(syncer) = &context.syncer else {
        return Err(RpcErr::Internal(
            "New payload requested but syncer is not initialized".to_string(),
        ));
    };
    // Validate block hash
    if let Err(RpcErr::Internal(error_msg)) = validate_block_hash(payload, &block) {
        return Ok(PayloadStatus::invalid_with_err(&error_msg));
    }

    // Check for invalid ancestors
    if let Some(status) = validate_ancestors(&block, &context).await? {
        return Ok(status);
    }

    // We have validated ancestors, the parent is correct
    let latest_valid_hash = block.header.parent_hash;

    if syncer.sync_mode() == SyncMode::Snap {
        debug!("Snap sync in progress, skipping new payload validation");
        return Ok(PayloadStatus::syncing());
    }

    // All checks passed, execute payload
    let payload_status = try_execute_payload(block, &context, latest_valid_hash).await?;
    Ok(payload_status)
}

async fn handle_new_payload_v3(
    payload: &ExecutionPayload,
    context: RpcApiContext,
    block: Block,
    expected_blob_versioned_hashes: Vec<H256>,
) -> Result<PayloadStatus, RpcErr> {
    // V3 specific: validate blob hashes
    let blob_versioned_hashes: Vec<H256> = block
        .body
        .transactions
        .iter()
        .flat_map(|tx| tx.blob_versioned_hashes())
        .collect();

    if expected_blob_versioned_hashes != blob_versioned_hashes {
        return Ok(PayloadStatus::invalid_with_err(
            "Invalid blob_versioned_hashes",
        ));
    }

    handle_new_payload_v1_v2(payload, block, context).await
}

async fn handle_new_payload_v4(
    payload: &ExecutionPayload,
    context: RpcApiContext,
    block: Block,
    expected_blob_versioned_hashes: Vec<H256>,
) -> Result<PayloadStatus, RpcErr> {
    // TODO: V4 specific: validate block access list
    handle_new_payload_v3(payload, context, block, expected_blob_versioned_hashes).await
}

// Elements of the list MUST be ordered by request_type in ascending order.
// Elements with empty request_data MUST be excluded from the list.
fn validate_execution_requests(execution_requests: &[EncodedRequests]) -> Result<(), RpcErr> {
    let mut last_type: i32 = -1;
    for requests in execution_requests {
        if requests.0.len() < 2 {
            return Err(RpcErr::WrongParam("Empty requests data.".to_string()));
        }
        let request_type = requests.0[0] as i32;
        if last_type >= request_type {
            return Err(RpcErr::WrongParam("Invalid requests order.".to_string()));
        }
        last_type = request_type;
    }
    Ok(())
}

fn get_block_from_payload(
    payload: &ExecutionPayload,
    parent_beacon_block_root: Option<H256>,
    requests_hash: Option<H256>,
    block_access_list_hash: Option<H256>,
) -> Result<Block, RLPDecodeError> {
    let block_hash = payload.block_hash;
    let block_number = payload.block_number;
    debug!(%block_hash, %block_number, "Received new payload");

    payload.clone().into_block(
        parent_beacon_block_root,
        requests_hash,
        block_access_list_hash,
    )
}

fn validate_block_hash(payload: &ExecutionPayload, block: &Block) -> Result<(), RpcErr> {
    let block_hash = payload.block_hash;
    let actual_block_hash = block.hash();
    if block_hash != actual_block_hash {
        return Err(RpcErr::Internal(format!(
            "Invalid block hash. Expected {actual_block_hash:#x}, got {block_hash:#x}"
        )));
    }
    Ok(())
}

async fn try_execute_payload(
    block: Block,
    context: &RpcApiContext,
    latest_valid_hash: H256,
) -> Result<PayloadStatus, RpcErr> {
    let Some(syncer) = &context.syncer else {
        return Err(RpcErr::Internal(
            "New payload requested but syncer is not initialized".to_string(),
        ));
    };
    let block_hash = block.hash();
    let block_number = block.header.number;
    let storage = &context.storage;
    // Return the valid message directly if we already have it.
    // We check for header only as we do not download the block bodies before the pivot during snap sync
    // https://github.com/lambdaclass/ethrex/issues/1766
    if storage.get_block_header_by_hash(block_hash)?.is_some() {
        return Ok(PayloadStatus::valid_with_hash(block_hash));
    }

    // Execute the block
    // We send the block to the block executor worker thread
    // This ensures blocks are executed sequentially and prevents blocking
    // the async runtime with CPU-intensive tasks
    debug!(%block_hash, %block_number, "Executing payload");
    match add_block(context, block, true).await {
        Err(ChainError::ParentNotFound) => {
            // Start sync
            syncer.sync_to_head(block_hash);
            Ok(PayloadStatus::syncing())
        }
        // Under the current implementation this is not possible: we always calculate the state
        // transition of any new payload as long as the parent is present. If we received the
        // parent payload but it was stashed, then new payload would stash this one too, with a
        // ParentNotFoundError.
        Err(ChainError::ParentStateNotFound) => {
            let e = "Failed to obtain parent state";
            error!("{e} for block {block_hash}");
            Err(RpcErr::Internal(e.to_string()))
        }
        Err(ChainError::InvalidBlock(error)) => {
            warn!("Error executing block: {error}");
            context
                .storage
                .set_latest_valid_ancestor(block_hash, latest_valid_hash)
                .await?;
            Ok(PayloadStatus::invalid_with(
                latest_valid_hash,
                error.to_string(),
            ))
        }
        Err(ChainError::EvmError(error)) => {
            warn!("Error executing block: {error}");
            context
                .storage
                .set_latest_valid_ancestor(block_hash, latest_valid_hash)
                .await?;
            Ok(PayloadStatus::invalid_with(
                latest_valid_hash,
                error.to_string(),
            ))
        }
        Err(ChainError::StoreError(error)) => {
            warn!("Error storing block: {error}");
            Err(RpcErr::Internal(error.to_string()))
        }
        Err(e) => {
            error!("{e} for block {block_hash}");
            Err(RpcErr::Internal(e.to_string()))
        }
        Ok(witness) => {
            debug!("Block with hash {block_hash} executed and added to storage successfully");
            let mut status = PayloadStatus::valid_with_hash(block_hash);
            if let Some(witness) = witness {
                let serialization_start = std::time::Instant::now();
                let rpc_witness = RpcExecutionWitness::try_from(witness)
                    .map_err(|e| RpcErr::Internal(e.to_string()))?;
                let witness_bytes = ethrex_rlp::encode::encode(rpc_witness);
                let witness_hex = format!("0x{}", hex::encode(witness_bytes));
                status.witness = Some(serde_json::Value::String(witness_hex));
                let serialization_duration = serialization_start.elapsed();
                info!(
                    "[witness-bench]: Block {} Witness serialization time: {:?}",
                    block_number, serialization_duration
                );
            }

            Ok(status)
        }
    }
}

pub async fn add_block(
    ctx: &RpcApiContext,
    block: Block,
    compute_witness: bool,
) -> Result<Option<ExecutionWitness>, ChainError> {
    let (tx, rx) = oneshot::channel();
    ctx.block_worker_channel
        .send((tx, block, compute_witness))
        .map_err(|_| ChainError::Custom("Block worker channel closed".to_string()))?;
    rx.await
        .map_err(|_| ChainError::Custom("Block worker dropped channel".to_string()))?
}
fn parse_get_payload_request(params: &Option<Vec<Value>>) -> Result<u64, RpcErr> {
    let params = params
        .as_ref()
        .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
    if params.len() != 1 {
        return Err(RpcErr::BadParams("Expected 1 param".to_owned()));
    };
    let Ok(hex_str) = serde_json::from_value::<String>(params[0].clone()) else {
        return Err(RpcErr::BadParams(
            "Expected param to be a string".to_owned(),
        ));
    };
    // Check that the hex string is 0x prefixed
    let Some(hex_str) = hex_str.strip_prefix("0x") else {
        return Err(RpcErr::BadHexFormat(0));
    };
    // Parse hex string
    let Ok(payload_id) = u64::from_str_radix(hex_str, 16) else {
        return Err(RpcErr::BadHexFormat(0));
    };
    Ok(payload_id)
}

fn validate_fork(block: &Block, fork: Fork, context: &RpcApiContext) -> Result<(), RpcErr> {
    // Check timestamp matches valid fork
    let chain_config = &context.storage.get_chain_config();
    let current_fork = chain_config.get_fork(block.header.timestamp);

    if current_fork != fork {
        return Err(RpcErr::UnsuportedFork(format!("{current_fork:?}")));
    }
    Ok(())
}

async fn get_payload(payload_id: u64, context: &RpcApiContext) -> Result<PayloadBundle, RpcErr> {
    info!(
        id = %format!("{:#018x}", payload_id),
        "Requested payload with"
    );
    let (blobs_bundle, requests, block_value, block, block_access_list) = {
        let PayloadBuildResult {
            blobs_bundle,
            block_value,
            requests,
            payload,
            block_access_list,
            ..
        } = context
            .blockchain
            .get_payload(payload_id)
            .await
            .map_err(|err| match err {
                ChainError::UnknownPayload => {
                    RpcErr::UnknownPayload(format!("Payload with id {payload_id:#018x} not found",))
                }
                err => RpcErr::Internal(err.to_string()),
            })?;
        (
            blobs_bundle,
            requests,
            block_value,
            payload,
            block_access_list,
        )
    };

    let new_payload = PayloadBundle {
        block,
        block_value,
        blobs_bundle,
        requests,
        block_access_list,
    };

    Ok(new_payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eth::gas_tip_estimator::GasTipEstimator;
    use crate::rpc::{ClientVersion, NodeData, RpcApiContext, start_block_executor};
    use crate::test_utils::{setup_store, test_header}; // removed default_context_with_storage
    use ethrex_blockchain::BlockchainOptions;
    use ethrex_common::types::DEFAULT_BUILDER_GAS_CEIL;
    use ethrex_common::types::{Block, BlockBody};
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    #[tokio::test]
    async fn test_new_payload_v3_returns_execution_witness() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();

        // 1. Setup store and context
        let store = setup_store().await;

        let mut opts = BlockchainOptions::default();
        opts.precompute_witnesses = false; // Ensure it's false to test the compute_witness flag plumbing
        let blockchain = Arc::new(ethrex_blockchain::Blockchain::new(store.clone(), opts));

        let block_worker_channel = start_block_executor(blockchain.clone());

        let local_node_record = crate::test_utils::example_local_node_record();

        let context = RpcApiContext {
            storage: store.clone(),
            blockchain: blockchain.clone(),
            active_filters: Default::default(),
            syncer: Some(Arc::new(crate::test_utils::dummy_sync_manager().await)),
            peer_handler: None,
            node_data: NodeData {
                jwt_secret: Default::default(),
                local_p2p_node: crate::test_utils::example_p2p_node(),
                local_node_record,
                client_version: ClientVersion::new(
                    "ethrex".to_string(),
                    "0.1.0".to_string(),
                    "test".to_string(),
                    "abcd1234".to_string(),
                    "x86_64-unknown-linux".to_string(),
                    "1.70.0".to_string(),
                ),
                extra_data: ethrex_common::Bytes::new(),
            },
            gas_tip_estimator: Arc::new(TokioMutex::new(GasTipEstimator::new())),
            log_filter_handler: None,
            gas_ceil: DEFAULT_BUILDER_GAS_CEIL,
            block_worker_channel,
        };

        let genesis_hash = store.get_canonical_block_hash(0).await.unwrap().unwrap();

        // 2. Create a valid block (genesis + 1)
        let block_header = test_header(1);
        let block_body = BlockBody {
            transactions: vec![],
            ommers: vec![],
            withdrawals: Some(vec![]),
        };
        let mut block = Block::new(block_header.clone(), block_body);

        // 3. Link to genesis and fix header fields to be valid
        let genesis_hash = store.get_canonical_block_hash(0).await.unwrap().unwrap();
        let genesis_header = store
            .get_block_header_by_hash(genesis_hash)
            .unwrap()
            .unwrap();

        block.header.parent_hash = genesis_hash;
        block.header.gas_limit = genesis_header.gas_limit; // Keep gas limit same as parent to avoid validation error
        block.header.timestamp = genesis_header.timestamp + 12; // Ensure timestamp is after parent (e.g. 1 slot)
        block.header.number = genesis_header.number + 1;

        // Fix base fee (EIP-1559)
        let expected_base_fee = ethrex_common::types::calculate_base_fee_per_gas(
            block.header.gas_limit,
            genesis_header.gas_limit,
            genesis_header.gas_used,
            genesis_header
                .base_fee_per_gas
                .unwrap_or(ethrex_common::types::INITIAL_BASE_FEE),
            ethrex_common::types::ELASTICITY_MULTIPLIER,
        )
        .expect("Failed to calculate base fee");

        block.header.base_fee_per_gas = Some(expected_base_fee);

        // 4. Build a valid block using PayloadBuildContext to set correct state root and other fields
        use ethrex_blockchain::payload::PayloadBuildContext;

        let mut build_context =
            PayloadBuildContext::new(block, &store, &context.blockchain.options.r#type)
                .expect("Failed to create PayloadBuildContext");

        // Apply system operations (beacon root etc) which updates state
        if let ethrex_blockchain::BlockchainType::L1 = context.blockchain.options.r#type {
            context
                .blockchain
                .apply_system_operations(&mut build_context)
                .expect("Failed to apply system operations");
        }

        context
            .blockchain
            .apply_withdrawals(&mut build_context)
            .expect("Failed to apply withdrawals");
        // No transactions to fill
        context
            .blockchain
            .extract_requests(&mut build_context)
            .expect("Failed to extract requests");

        context
            .blockchain
            .finalize_payload(&mut build_context)
            .expect("Failed to finalize payload");

        let block = build_context.payload;

        // Updated block has correct state root, gas used, etc.

        // Ensure blockchain thinks it is synced to generate witness
        context.blockchain.set_synced();

        // 5. Call try_execute_payload
        // We use genesis_hash as latest_valid_hash
        let result = try_execute_payload(block, &context, genesis_hash).await;

        // 5. Assertions
        assert!(result.is_ok(), "Execution failed: {:?}", result.err());
        let status = result.unwrap();

        // Verify status is Valid
        // Note: PayloadStatus::valid_with_hash might be what is returned
        match status.status {
            crate::types::payload::PayloadValidationStatus::Valid => {}
            _ => panic!("Payload status not valid: {:?}", status),
        }

        // 6. Verify witness is present
        assert!(
            status.witness.is_some(),
            "Witness field is missing in PayloadStatus. Status: {:?}",
            status
        );

        // 7. Validate witness format
        let witness_json = status.witness.unwrap();

        // Expect a hex string
        assert!(witness_json.is_string(), "Witness should be a hex string");
        let witness_str = witness_json.as_str().unwrap();
        assert!(
            witness_str.starts_with("0x"),
            "Witness hex string should start with 0x"
        );

        // Decode hex
        let witness_bytes = hex::decode(&witness_str[2..]).expect("Failed to decode hex witness");
        assert!(
            !witness_bytes.is_empty(),
            "Witness bytes should not be empty"
        );
    }
}
