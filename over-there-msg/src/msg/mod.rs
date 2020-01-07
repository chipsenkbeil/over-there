pub mod types;

use chrono::prelude::{DateTime, Utc};
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Msg {
    /// ID associated with a request or response
    pub id: u32,

    /// The time at which the message was created
    pub creation_date: DateTime<Utc>,

    /// Content within the message
    content: Box<dyn types::Content>,
}

impl Msg {
    pub fn from_content<T: types::Content + 'static>(content: T) -> Self {
        Self {
            id: random(),
            creation_date: Utc::now(),
            content: Box::new(content),
        }
    }

    pub fn is_content<T: types::Content + 'static>(&self) -> bool {
        self.content.as_any().is::<T>()
    }

    pub fn to_content<T: types::Content + 'static>(&self) -> Option<&T> {
        self.content.as_any().downcast_ref::<T>()
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    enum Content1 {
        Value1(String),
        Value2(u32),
    }

    #[typetag::serde]
    impl types::Content for Content1 {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    enum Content2 {
        Value3(bool),
    }

    #[typetag::serde]
    impl types::Content for Content2 {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn is_content_should_yield_false_if_incorrect_concrete_type() {
        let content = Content1::Value1(String::from("test"));
        let msg = Msg::from_content(content);

        assert!(!msg.is_content::<Content2>());
    }

    #[test]
    fn is_content_should_yield_true_if_correct_concrete_type() {
        let content = Content1::Value1(String::from("test"));
        let msg = Msg::from_content(content);

        assert!(msg.is_content::<Content1>());
    }

    #[test]
    fn to_content_should_yield_none_if_incorrect_concrete_type() {
        let content = Content1::Value1(String::from("test"));
        let msg = Msg::from_content(content);

        assert!(msg.to_content::<Content2>().is_none());
    }

    #[test]
    fn to_content_should_yield_some_content_if_correct_concrete_type() {
        let content = Content1::Value1(String::from("test"));
        let msg = Msg::from_content(content);

        let cast_content = msg
            .to_content::<Content1>()
            .expect("Failed to cast content");
        match cast_content {
            Content1::Value1(x) => assert_eq!(x, "test", "Content value was wrong"),
            x => panic!("Unexpected content: {:?}", x),
        }
    }

    #[test]
    fn from_content_should_produce_a_new_msg_with_that_content() {
        // Try a content type
        let content = Content1::Value1(String::from("test"));
        let msg = Msg::from_content(content);
        assert!(
            Utc::now()
                .signed_duration_since(msg.creation_date)
                .num_milliseconds()
                >= 0,
            "Unexpected creation date: {:?}",
            msg.creation_date
        );
        match msg
            .to_content::<Content1>()
            .expect("Unable to cast content")
        {
            Content1::Value1(x) => assert_eq!(x, "test", "Content value was incorrect"),
            x => panic!("Unexpected content: {:?}", x),
        }

        // Now try different content type
        let content = Content2::Value3(true);
        let msg = Msg::from_content(content);
        assert!(
            Utc::now()
                .signed_duration_since(msg.creation_date)
                .num_milliseconds()
                >= 0,
            "Unexpected creation date: {:?}",
            msg.creation_date
        );
        let Content2::Value3(x) = msg
            .to_content::<Content2>()
            .expect("Unable to cast content");
        assert!(x, "Content value was incorrect");
    }
}
