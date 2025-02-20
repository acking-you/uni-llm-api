use axum::{extract::State, Json};
use serde::Serialize;

use crate::SharedStateRef;

#[derive(Debug, Serialize)]
pub(crate) struct ModelInfoResp {
    /// A unique name used to identify the calling model,
    /// corresponding to the key in [`crate::UniModelsInfo::models`]
    name: String,
    model: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ApiTagsResponse {
    models: Vec<ModelInfoResp>,
}

pub(crate) async fn api_tags(
    State(state): State<SharedStateRef>,
) -> Json<ApiTagsResponse> {
    let models = {
        let guard = state.model_config.read();
        guard
            .models
            .keys()
            .map(|v| ModelInfoResp {
                name: v.to_string(),
                model: v.to_string(),
            })
            .collect()
    };
    Json(ApiTagsResponse { models })
}
