use super::Content;
use jsonpath_lib as jsonpath;
use over_there_derive::Error;
use serde::{Deserialize, Serialize};

/// Represents an error that can occur when transforming content based on
/// prior results from a sequential operation
#[derive(Debug, Error)]
pub enum TransformContentError {
    ContentToJsonFailed(serde_json::Error),
    JsonToContentFailed(serde_json::Error),
    ExtractingBaseValueFailed(jsonpath::JsonPathError),
    BaseValueMissing { path: String },
    BaseValueNotScalar { path: String },
    ReplacementFailed(jsonpath::JsonPathError),
}

/// Represents content that will be transformed at runtime based on some
/// prior input
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LazilyTransformedContent {
    /// Represents collection of transformation rules to apply to raw content
    pub rules: Vec<TransformRule>,

    /// Represents content prior to being transformed
    pub raw_content: Content,
}

impl LazilyTransformedContent {
    /// Converts to the raw content with no transformations applied
    pub fn into_raw_content(self) -> Content {
        self.raw_content
    }

    /// Performs the transformation of content by applying all rules in order
    /// and returning the resulting content
    pub fn transform_with_base(
        &self,
        base: &Content,
    ) -> Result<Content, TransformContentError> {
        let mut value = serde_json::to_value(&self.raw_content)
            .map_err(TransformContentError::ContentToJsonFailed)?;
        let base_value = serde_json::to_value(base)
            .map_err(TransformContentError::ContentToJsonFailed)?;

        for rule in self.rules.iter() {
            // For now, we're assuming that the replacement value must be
            // a singular value (not replacing with an array, object, etc)
            let mut new_values = jsonpath::select(&base_value, &rule.value)
                .map_err(TransformContentError::ExtractingBaseValueFailed)?;
            if new_values.is_empty() {
                return Err(TransformContentError::BaseValueMissing {
                    path: rule.value.clone(),
                });
            } else if new_values.len() > 1 {
                return Err(TransformContentError::BaseValueNotScalar {
                    path: rule.value.clone(),
                });
            }
            let new_value = new_values.drain(0..=0).last();

            value = jsonpath::replace_with(value, &rule.path, &mut |_| {
                new_value.cloned()
            })
            .map_err(TransformContentError::ReplacementFailed)?;
        }

        serde_json::from_value(value)
            .map_err(TransformContentError::JsonToContentFailed)
    }
}

/// Represents a transformation to apply against some content; uses syntax
/// like JSONPath in that $.field can be used to reference the fields of the
/// objects
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TransformRule {
    /// Represents the key (at a JSON level) to transform for some content;
    /// this will be interpolated using $ to represent the root of the current
    /// object (content)
    pub path: String,

    /// Represents the new value to apply to the key; this will be interpolated
    /// based on a previous result if present using $ to represent the root
    /// of the previous output content as a JSON object and dot notation for
    /// the nested keys
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::super::{CustomArgs, FileOpenedArgs};
    use super::*;

    #[test]
    fn transform_with_base_should_fail_if_rule_value_not_found() {
        let raw_content = Content::DoReadFile(Default::default());
        let base_content = Content::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_content = LazilyTransformedContent {
            raw_content: raw_content.clone(),
            rules: vec![TransformRule {
                // Replace id of raw content
                path: String::from("$.id"),

                // Apply missing field from base content
                value: String::from("$.missing_field"),
            }],
        };

        match lazy_content.transform_with_base(&base_content) {
            Err(TransformContentError::BaseValueMissing { .. }) => (),
            x => panic!("Unexpected content: {:?}", x),
        }
    }

    #[test]
    fn transform_with_base_should_fail_if_rule_value_not_scalar() {
        let raw_content = Content::DoReadFile(Default::default());
        let base_content = Content::Custom(CustomArgs {
            data: vec![0, 1, 2],
        });

        let lazy_content = LazilyTransformedContent {
            raw_content: raw_content.clone(),
            rules: vec![TransformRule {
                // Replace id of raw content
                path: String::from("$.id"),

                // Apply array data field from base content
                value: String::from("$.data[*]"),
            }],
        };

        match lazy_content.transform_with_base(&base_content) {
            Err(TransformContentError::BaseValueNotScalar { .. }) => (),
            x => panic!("Unexpected content: {:?}", x),
        }
    }

    #[test]
    fn transform_with_base_should_fail_if_rule_value_not_same_type_as_path() {
        let raw_content = Content::Error(Default::default());
        let base_content = Content::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_content = LazilyTransformedContent {
            raw_content: raw_content.clone(),
            rules: vec![TransformRule {
                // Replace msg of raw content
                path: String::from("$.msg"),

                // Apply id from base content
                value: String::from("$.id"),
            }],
        };

        match lazy_content.transform_with_base(&base_content) {
            Err(TransformContentError::JsonToContentFailed(_)) => (),
            x => panic!("Unexpected content: {:?}", x),
        }
    }

    #[test]
    fn transform_with_base_should_return_raw_content_if_rule_path_missing() {
        let raw_content = Content::DoReadFile(Default::default());
        let base_content = Content::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_content = LazilyTransformedContent {
            raw_content: raw_content.clone(),
            rules: vec![TransformRule {
                // Replace missing field of raw content
                path: String::from("$.missing_field"),

                // Apply id from base content
                value: String::from("$.id"),
            }],
        };

        match lazy_content.transform_with_base(&base_content) {
            Ok(content) => {
                assert_eq!(content, raw_content, "Raw content altered")
            }
            x => panic!("Unexpected content: {:?}", x),
        }
    }

    #[test]
    fn transform_with_base_should_succeed_if_able_to_replace_path_with_value() {
        let raw_content = Content::DoReadFile(Default::default());
        let base_content = Content::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_content = LazilyTransformedContent {
            raw_content: raw_content.clone(),
            rules: vec![TransformRule {
                // Replace id of raw content
                path: String::from("$.id"),

                // Apply id from base content
                value: String::from("$.id"),
            }],
        };

        let transformed_content = lazy_content
            .transform_with_base(&base_content)
            .expect("Failed to transform");

        match transformed_content {
            Content::DoReadFile(args) => {
                assert_eq!(args.id, 123);
                assert_ne!(args.sig, 456);
            }
            x => panic!("Unexpected content: {:?}", x),
        }
    }

    #[test]
    fn transform_with_base_should_apply_rules_in_sequence() {
        let raw_content = Content::DoReadFile(Default::default());
        let base_content = Content::FileOpened(FileOpenedArgs {
            id: 123,
            sig: 456,
            ..Default::default()
        });

        let lazy_content = LazilyTransformedContent {
            raw_content: raw_content.clone(),
            rules: vec![
                TransformRule {
                    // Replace id of raw content
                    path: String::from("$.id"),

                    // Apply id from base content
                    value: String::from("$.id"),
                },
                TransformRule {
                    // Replace sig of raw content
                    path: String::from("$.sig"),

                    // Apply sig from base content
                    value: String::from("$.sig"),
                },
            ],
        };

        let transformed_content = lazy_content
            .transform_with_base(&base_content)
            .expect("Failed to transform");

        match transformed_content {
            Content::DoReadFile(args) => {
                assert_eq!(args.id, 123);
                assert_eq!(args.sig, 456);
            }
            x => panic!("Unexpected content: {:?}", x),
        }
    }
}
