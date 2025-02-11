use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ModelInfoResp {
    /// A unique name used to identify the calling model,
    /// corresponding to the key in [`super::UniModelsInfo::models`]
    name: String,
}

#[derive(Debug, Serialize)]
pub struct ApiTagsResponse {
    models: Vec<ModelInfoResp>,
}

pub async fn api_tags() -> Json<ApiTagsResponse> {
    Json(ApiTagsResponse {
        models: vec![ModelInfoResp {
            name: "aliyun/qwen-max".to_string(),
        }],
    })
}
