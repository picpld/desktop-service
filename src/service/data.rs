use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StartBody {
    pub bin_path: String,
    pub args: Vec<String>,
    pub log_file: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DnsBody {
    pub dns: String,
}

#[derive(Deserialize, Serialize)]
pub struct JsonResponse<T: Serialize> {
    pub code: u64,
    pub msg: String,
    pub data: Option<T>,
}
