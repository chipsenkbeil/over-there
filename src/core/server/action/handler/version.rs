use crate::reply::VersionArgs;
use log::debug;

pub async fn version() -> VersionArgs {
    debug!("version_request");
    VersionArgs {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn version_should_return_version() {
        let args = version().await;

        assert_eq!(args.version, env!("CARGO_PKG_VERSION").to_string());
    }
}
