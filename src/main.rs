#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use log::error;

    if let Err(e) = pokertimer::backend::main().await {
        error!("Uncaught error {e}");
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
