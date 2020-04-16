use crate::{Reply, Request};
use jsonpath_lib as jsonpath;
use over_there_derive::Error;
use serde::{Deserialize, Serialize};

/// Represents an error that can occur when transforming request replyd on
/// prior results from a sequential operation
#[derive(Debug, Error)]
pub enum TransformRequestError {
    RequestToJsonFailed(serde_json::Error),
    JsonToRequestFailed(serde_json::Error),
    ExtractingReplyValueFailed(jsonpath::JsonPathError),
    ReplyValueMissing { path: String },
    ReplyValueNotScalar { path: String },
    ReplacementFailed(jsonpath::JsonPathError),
}

/// Represents request that will be transformed at runtime replyd on some
/// prior input
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LazilyTransformedRequest {
    /// Represents collection of transformation rules to apply to raw request
    pub rules: Vec<TransformRule>,

    /// Represents request prior to being transformed
    pub raw_request: Request,
}

impl LazilyTransformedRequest {
    pub fn new(raw_request: Request, rules: Vec<TransformRule>) -> Self {
        Self { rules, raw_request }
    }

    /// Converts to the raw request with no transformations applied
    pub fn into_raw_request(self) -> Request {
        self.raw_request
    }

    /// Performs the transformation of request by applying all rules in order
    /// and returning the resulting request
    pub fn transform_with_reply(
        &self,
        reply: &Reply,
    ) -> Result<Request, TransformRequestError> {
        let mut value = serde_json::to_value(&self.raw_request)
            .map_err(TransformRequestError::RequestToJsonFailed)?;
        let reply_value = serde_json::to_value(reply)
            .map_err(TransformRequestError::RequestToJsonFailed)?;

        for rule in self.rules.iter() {
            // For now, we're assuming that the replacement value must be
            // a singular value (not replacing with an array, object, etc)
            let mut new_values = jsonpath::select(&reply_value, &rule.value)
                .map_err(TransformRequestError::ExtractingReplyValueFailed)?;
            if new_values.is_empty() {
                return Err(TransformRequestError::ReplyValueMissing {
                    path: rule.value.clone(),
                });
            } else if new_values.len() > 1 {
                return Err(TransformRequestError::ReplyValueNotScalar {
                    path: rule.value.clone(),
                });
            }
            let new_value = new_values.drain(0..=0).last();

            value = jsonpath::replace_with(value, &rule.path, &mut |_| {
                new_value.cloned()
            })
            .map_err(TransformRequestError::ReplacementFailed)?;
        }

        serde_json::from_value(value)
            .map_err(TransformRequestError::JsonToRequestFailed)
    }
}

/// Represents a transformation to apply against some request; uses syntax
/// like JSONPath in that $.field can be used to reference the fields of the
/// objects
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TransformRule {
    /// Represents the key (at a JSON level) to transform for some request;
    /// this will be interpolated using $ to represent the root of the current
    /// object (request)
    pub path: String,

    /// Represents the new value to apply to the key; this will be interpolated
    /// replyd on a previous result if present using $ to represent the root
    /// of the previous output request as a JSON object and dot notation for
    /// the nested keys
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reply::{CustomArgs, FileOpenedArgs};

    #[test]
    fn transform_with_reply_should_fail_if_rule_value_not_found() {
        let raw_request = Request::ReadFile(Default::default());
        let reply = Reply::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_request = LazilyTransformedRequest {
            raw_request: raw_request.clone(),
            rules: vec![TransformRule {
                // Replace id of raw request
                path: String::from("$.payload.id"),

                // Apply missing field from reply request
                value: String::from("$.payload.missing_field"),
            }],
        };

        match lazy_request.transform_with_reply(&reply) {
            Err(TransformRequestError::ReplyValueMissing { .. }) => (),
            x => panic!("Unexpected request: {:?}", x),
        }
    }

    #[test]
    fn transform_with_reply_should_fail_if_rule_value_not_scalar() {
        let raw_request = Request::ReadFile(Default::default());
        let reply = Reply::Custom(CustomArgs {
            data: vec![0, 1, 2],
        });

        let lazy_request = LazilyTransformedRequest {
            raw_request: raw_request.clone(),
            rules: vec![TransformRule {
                // Replace id of raw request
                path: String::from("$.payload.id"),

                // Apply array data field from reply request
                value: String::from("$.payload.data[*]"),
            }],
        };

        match lazy_request.transform_with_reply(&reply) {
            Err(TransformRequestError::ReplyValueNotScalar { .. }) => (),
            x => panic!("Unexpected request: {:?}", x),
        }
    }

    #[test]
    fn transform_with_reply_should_fail_if_rule_value_not_same_type_as_path() {
        let raw_request = Request::Custom(Default::default());
        let reply = Reply::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_request = LazilyTransformedRequest {
            raw_request: raw_request.clone(),
            rules: vec![TransformRule {
                // Replace data of raw request
                path: String::from("$.payload.data"),

                // Apply id from reply request
                value: String::from("$.payload.id"),
            }],
        };

        match lazy_request.transform_with_reply(&reply) {
            Err(TransformRequestError::JsonToRequestFailed(_)) => (),
            x => panic!("Unexpected request: {:?}", x),
        }
    }

    #[test]
    fn transform_with_reply_should_return_raw_request_if_rule_path_missing() {
        let raw_request = Request::ReadFile(Default::default());
        let reply = Reply::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_request = LazilyTransformedRequest {
            raw_request: raw_request.clone(),
            rules: vec![TransformRule {
                // Replace missing field of raw request
                path: String::from("$.payload.missing_field"),

                // Apply id from reply request
                value: String::from("$.payload.id"),
            }],
        };

        match lazy_request.transform_with_reply(&reply) {
            Ok(request) => {
                assert_eq!(request, raw_request, "Raw request altered")
            }
            x => panic!("Unexpected request: {:?}", x),
        }
    }

    #[test]
    fn transform_with_reply_should_succeed_if_able_to_replace_path_with_value()
    {
        let raw_request = Request::ReadFile(Default::default());
        let reply = Reply::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_request = LazilyTransformedRequest {
            raw_request: raw_request.clone(),
            rules: vec![TransformRule {
                // Replace id of raw request
                path: String::from("$.payload.id"),

                // Apply id from reply request
                value: String::from("$.payload.id"),
            }],
        };

        let transformed_request = lazy_request
            .transform_with_reply(&reply)
            .expect("Failed to transform");

        match transformed_request {
            Request::ReadFile(args) => {
                assert_eq!(args.id, 123);
                assert_ne!(args.sig, 456);
            }
            x => panic!("Unexpected request: {:?}", x),
        }
    }

    #[test]
    fn transform_with_reply_should_apply_rules_in_sequence() {
        let raw_request = Request::ReadFile(Default::default());
        let reply = Reply::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_request = LazilyTransformedRequest {
            raw_request: raw_request.clone(),
            rules: vec![
                TransformRule {
                    // Replace id of raw request
                    path: String::from("$.payload.id"),

                    // Apply id from reply request
                    value: String::from("$.payload.id"),
                },
                TransformRule {
                    // Replace sig of raw request
                    path: String::from("$.payload.sig"),

                    // Apply sig from reply request
                    value: String::from("$.payload.sig"),
                },
            ],
        };

        let transformed_request = lazy_request
            .transform_with_reply(&reply)
            .expect("Failed to transform");

        match transformed_request {
            Request::ReadFile(args) => {
                assert_eq!(args.id, 123);
                assert_eq!(args.sig, 456);
            }
            x => panic!("Unexpected request: {:?}", x),
        }
    }
}
