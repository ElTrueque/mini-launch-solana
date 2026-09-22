FROM rust:1.89.0-slim-bookworm@sha256:d7fc7de78bb8c1469933aeecbf801314d30d7d6e9f0578bba4cfa285bfa37fe6
RUN apt-get update && apt-get install -y --no-install-recommends python3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY reproduce.py /opt/mini-launch-tools/reproduce.py
RUN python3 /opt/mini-launch-tools/reproduce.py --download-tools --prepare-only && \
    rm /opt/mini-launch-tools/.build/platform-tools-*.tar.bz2
COPY cargo-build-sbf /usr/local/bin/cargo-build-sbf
RUN chmod 755 /usr/local/bin/cargo-build-sbf
WORKDIR /build
LABEL org.opencontainers.image.source="https://github.com/ElTrueque/mini-launch-solana"
LABEL org.opencontainers.image.description="Pinned compiler for reproducing the original Mini Launch executable from mounted source; contains no precompiled Mini Launch program."
