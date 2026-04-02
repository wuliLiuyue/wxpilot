use wxp_daemon::bootstrap_guard::{run_bootstrap_guard, AllowAllBootstrapGuard};
use wxp_daemon::server::{serve_forever, RuntimePaths, ServerState};

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

    let state = match ServerState::new(RuntimePaths::default()) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("rust daemon init error: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = serve_forever(state) {
        eprintln!("rust daemon error: {error}");
        std::process::exit(1);
    }
}
