//! Snippets for `reference/feature-flags` (also used by `reference/troubleshooting-and-faq`).

/// Installs the `ring` rustls crypto provider for the whole process.
///
/// Swiftide's HTTP clients use rustls without choosing a crypto provider, so an application must
/// install one before it builds the first client.
pub fn install_crypto_provider() {
    // ANCHOR: crypto_provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("a rustls crypto provider was already installed");
    // ANCHOR_END: crypto_provider
}
