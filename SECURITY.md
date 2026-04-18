# Security policy

## Reporting

Please report security vulnerabilities **privately** using the contact methods embedded in the on-chain program’s security.txt (see Solana Explorer → your program → Security), or update the addresses in `programs/colosseum_prediction/src/security_contact.rs` before deployment and use those channels.

Do not exploit issues against production users without prior written agreement.

## Bug bounty

Unless stated elsewhere by the maintainers, **no bug bounty is guaranteed**; reports are still appreciated.

## Embedded metadata

Contact details for explorers are defined in `programs/colosseum_prediction/src/security_contact.rs` via the `solana-security-txt` crate. After changing them, rebuild and redeploy the program.
