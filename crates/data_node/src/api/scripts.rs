use crate::client::DataNode;
use bf_api_provider::types::{
    ScriptsCborResponse, ScriptsDatumCborResponse, ScriptsDatumResponse, ScriptsJsonResponse,
    ScriptsSingleResponse,
};
use bf_common::types::ApiResult;

pub struct DataNodeScripts<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn scripts(&self) -> DataNodeScripts<'_> {
        DataNodeScripts { inner: self }
    }
}

impl DataNodeScripts<'_> {
    pub async fn by(&self, script_hash: &str) -> ApiResult<ScriptsSingleResponse> {
        let path = format!("scripts/{script_hash}");

        self.inner.client.get(&path, None).await
    }

    pub async fn json(&self, script_hash: &str) -> ApiResult<ScriptsJsonResponse> {
        let path = format!("scripts/{script_hash}/json");

        self.inner.client.get(&path, None).await
    }

    pub async fn cbor(&self, script_hash: &str) -> ApiResult<ScriptsCborResponse> {
        let path = format!("scripts/{script_hash}/cbor");

        self.inner.client.get(&path, None).await
    }

    pub async fn datum(&self, datum_hash: &str) -> ApiResult<ScriptsDatumResponse> {
        let path = format!("scripts/datum/{datum_hash}");

        self.inner.client.get(&path, None).await
    }

    pub async fn datum_cbor(&self, datum_hash: &str) -> ApiResult<ScriptsDatumCborResponse> {
        let path = format!("scripts/datum/{datum_hash}/cbor");

        self.inner.client.get(&path, None).await
    }
}
