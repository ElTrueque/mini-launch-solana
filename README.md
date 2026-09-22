# Mini Launch by El Trueque â€” Solana

Source snapshot for the Mini Launch program deployed on Solana mainnet-beta.

- Program: `5B4bmFyPrTXE9FFynQMP5idgyJqNJ9DHdfQQgHnQ1e62`
- ProgramData: `ELoBDEwspRRMezTrCZph4tg6ux2wHQ82EadcZb2WMPhw`
- Original executable size: **92,072 bytes**
- Original executable SHA-256: `5424ba2462eb78826c7a4e7f8dacc2d11fc8cffc1bcff8ac33db215db4f132fb`
- Compiler: Anza platform-tools **v1.57**, Rust `1.95.0-dev (ae660768a 2026-08-17)`
- Target: `sbpfv3-solana-solana`

## Verification status

The original source files and lockfile have been preserved byte-for-byte. Fresh Windows and Linux builds reproduce the deployed executable exactly. The successful independent Linux run is recorded in [GitHub Actions](https://github.com/ElTrueque/mini-launch-solana/actions/runs/35676758376).

**Public OtterSec verification is not complete.** Publishing source and passing a reproducible build are not themselves an explorer verification or an independent security audit. The container integration and public verification registration are the remaining steps.

This repository does not contain wallet keys, credentials, deployment accounts or a deploy command. Running the build does not sign transactions or modify the on-chain program.

## Reproduce the original build

Python 3.12 or newer is required. Use official Anza platform-tools v1.57, extracted into a directory containing `rust/bin` and `llvm/bin`:

```text
python reproduce.py --platform-tools /path/to/platform-tools
```

Alternatively, download the platform-tools release and verify its pinned SHA-256 automatically:

```text
python reproduce.py --download-tools
```

The script fetches only the six dependencies in `program/Cargo.lock`, checks their registry checksums, builds with the original release settings, and fails if the resulting executable hash is different. Windows downloads approximately 580 MB of tooling; Linux downloads both pinned releases (approximately 1.1 GB total) to use the native compiler with the original portable SBPF target libraries. Build files stay under `.build/` and are ignored by Git.

The original executable includes Windows diagnostic paths. The recipe remaps source paths to those same strings and uses the SBPF standard libraries from the original Windows platform-tools release. These target libraries compile successfully with the Linux compiler from the same release. The recipe does not edit or patch the compiled executable. The path strings already exist in the publicly deployed program.

## Container integration

The Dockerfile packages only the pinned build tools. It contains no compiled Mini Launch executable. Its `cargo-build-sbf` adapter recompiles the mounted repository using the original direct Cargo procedure and preserves the unstripped executable, as originally deployed. It refuses unsupported build overrides and checks the full executable hash before copying the newly built file into the verifier's expected output directory.

```sh
docker build -t mini-launch-builder .
solana-verify build --base-image mini-launch-builder "$PWD" --workspace-path "$PWD/program" --library-name mini_launch_solana --arch v3
```

These commands build locally. They do not upload a verification record, deploy a program, or sign a transaction.

## Source layout

- `program/src/lib.rs`: launch creation, fixed-price trading and launch state.
- `program/src/graduation.rs`: Raydium graduation and reserve settlement.
- `program/src/fees.rs`: fee distribution.
- `program/src/ids.rs`: public program and treasury addresses.
- `program/src/sol.rs`: Solana helpers.
- `program/Cargo.toml` and `program/Cargo.lock`: original build settings and pinned dependencies.

Official website: https://el-trueque.com
