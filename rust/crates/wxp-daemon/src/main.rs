use wxp_daemon::bootstrap_guard::{run_bootstrap_guard, AllowAllBootstrapGuard};
use wxp_daemon::server::{serve_forever, ServerState};

fn main() {
    let decision = run_bootstrap_guard(&AllowAllBootstrapGuard);
    if !decision.allowed {
        eprintln!(
            "{}",
            decision
                .reason
                .as_deref()
                .unwrap_or("bootstrap guard rejected startup")
        );
        std::process::exit(1);
    }

    if let Err(error) = serve_forever(ServerState::default()) {
        eprintln!("rust daemon error: {error}");
        std::process::exit(1);
    }
}
