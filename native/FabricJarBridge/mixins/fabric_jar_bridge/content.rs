fn fabric_bridge_content(
    root: &std::path::Path,
    enabled: &std::collections::BTreeSet<String>,
    dev: &[std::path::PathBuf],
) -> ::core::ops::ControlFlow<std::path::PathBuf> {
    ::core::ops::ControlFlow::Break(crate::game::fabric_jar_bridge::content_root(root, enabled, dev))
}
