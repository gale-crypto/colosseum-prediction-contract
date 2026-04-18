//! Embedded [security.txt](https://securitytxt.org/) for explorers (Solana Explorer, Solscan, etc.).
//!
//! **Before mainnet:** edit the string literals in `security_txt!` below — especially
//! `project_url`, `contacts`, and `policy` — then rebuild and redeploy. Validate with:
//! `cargo install query-security-txt && query-security-txt target/deploy/colosseum_prediction.so`

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

// `no-entrypoint` is enabled for CPI crates (`features = ["cpi"]`); skip embedding there.
#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Colosseum Prediction",
    // Replace with your live site, e.g. "https://your-dapp.com" (must be https or use "private").
    project_url: "private",
    // Replace `.invalid` with a real monitored address (RFC 2606 placeholder — mail will not route).
    contacts: "email:security@colosseum-prediction.invalid,link:https://example.com/security",
    // Replace with your published policy URL once available, or keep this short inline policy.
    policy: "Responsible disclosure: report security issues to the email in contacts only; do not exploit active systems without written permission. No bug bounty is implied unless published separately by the project maintainers.",
    preferred_languages: "en"
}
