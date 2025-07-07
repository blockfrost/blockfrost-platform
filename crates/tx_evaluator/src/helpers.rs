use bf_common::{config::Evaluator, errors::BlockfrostError};

use crate::external::ExternalEvaluator;

// query param overrides the config
pub fn is_external_evaluator(
    query_param: Option<String>,
    evaluator_config: &Evaluator,
    external_evaluator_opt: &Option<ExternalEvaluator>,
) -> Result<bool, BlockfrostError> {
    match query_param {
        Some(v) => {
            if Evaluator::try_from(v)? == Evaluator::External {
                if external_evaluator_opt.is_none() {
                    Err(BlockfrostError::custom_400(
                        "External validator is not enabled in the config".to_string(),
                    ))
                } else {
                    Ok(true)
                }
            } else {
                Ok(false)
            }
        },
        None => Ok(evaluator_config == &Evaluator::External),
    }
}
