# Mini Launch by El Trueque — Solana

Source snapshot for the Mini Launch program deployed on Solana mainnet-beta.

- Program: `5B4bmFyPrTXE9FFynQMP5idgyJqNJ9DHdfQQgHnQ1e62`
- ProgramData: `ELoBDEwspRRMezTrCZph4tg6ux2wHQ82EadcZb2WMPhw`
- Original executable size: **92,072 bytes**
- Original executable SHA-256: `5424ba2462eb78826c7a4e7f8dacc2d11fc8cffc1bcff8ac33db215db4f132fb`
- Compiler: Anza platform-tools **v1.57**, Rust `1.95.0-dev (ae660768a 2026-08-17)`
- Target: `sbpfv3-solana-solana`

## Verification status

The original source files and lockfile have been preserved byte-for-byte. A fresh Windows build from a separate directory reproduces the deployed executable exactly after restoring the original compiler diagnostic path strings.

**Public OtterSec verification is not complete.** Publishing source is not itself an explorer verification or an independent security audit. Linux/container reproduction and the public verification service must still be checked.

This repository does not contain wallet keys, credentials, deployment accounts or a deploy command. Running the build does not sign transactions or modify the on-chain program.

## Reproduce the original build

Python 3.11 or newer is required. Use official Anza platform-tools v1.57, extracted into a directory containing `rust/bin` and `llvm/bin`:

```text
python reproduce.py --platform-tools /path/to/platform-tools
```

Alternatively, download the platform-tools release and verify its pinned SHA-256 automatically:

```text
python reproduce.py --download-tools
```

The script fetches only the six dependencies in `program/Cargo.lock`, checks their registry checksums, builds with the original release settings, and fails if the resulting executable hash is different. The platform-tools download is approximately 530–580 MB. Build files stay under `.build/` and are ignored by Git.

The original executable includes Windows diagnostic paths. The recipe remaps dependency source paths to those same strings; it does not edit or patch the compiled executable. These strings already exist in the publicly deployed program.

## Source layout

- `program/src/lib.rs`: launch creation, fixed-price trading and launch state.
- `program/src/graduation.rs`: Raydium graduation and reserve settlement.
- `program/src/fees.rs`: fee distribution.
- `program/src/ids.rs`: public program and treasury addresses.
- `program/src/sol.rs`: Solana helpers.
- `program/Cargo.toml` and `program/Cargo.lock`: original build settings and pinned dependencies.

Official website: https://el-trueque.com
