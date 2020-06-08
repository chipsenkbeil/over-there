use crate::core::reply::{CapabilitiesArgs, Capability};
use log::debug;

pub async fn capabilities() -> CapabilitiesArgs {
    debug!("handler::capabilities");
    CapabilitiesArgs {
        capabilities: vec![
            #[cfg(feature = "custom")]
            Capability::Custom,
            #[cfg(feature = "exec")]
            Capability::Exec,
            #[cfg(feature = "file-system")]
            Capability::FileSystem,
            #[cfg(feature = "forward")]
            Capability::Forward,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capabilities_should_return_capabilities() {
        let results = capabilities().await;

        assert_eq!(
            results.capabilities,
            vec![
                Capability::Custom,
                Capability::Exec,
                Capability::FileSystem,
                Capability::Forward
            ],
        );
    }
}
