use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(pub Uuid);

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRequest {
    pub id: RequestId,
    pub method: JsonRequestMethod,
    pub path: String,
    pub header: Vec<JsonHeader>,
    pub body_base64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonResponse {
    pub id: RequestId,
    pub code: u16,
    pub header: Vec<JsonHeader>,
    pub body_base64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonHeader {
    pub name: String,
    pub value: String,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Debug)]
pub enum JsonRequestMethod {
    GET,
    POST,
}
