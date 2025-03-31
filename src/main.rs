#[cfg(feature = "ssr")]
// We use current_thread here because this is running on production on a
// single vcpu box and this makes the dev environment closer to the production
// one
#[tokio::main(flavor = "current_thread")]
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
