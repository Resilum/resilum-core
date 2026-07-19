# syntax=docker/dockerfile:1
#
# Evaluation environment: builds both candidate Rust Reticulum stacks and runs
# their bundled examples/binaries so we can judge which fits resilum-core.
# glibc base (not musl) to maximize build success while evaluating.

FROM rust:1-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
        git pkg-config libssl-dev clang cmake \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /eval
COPY eval.sh /eval/eval.sh
RUN chmod +x /eval/eval.sh
# Network is required at run time (clone + fetch crates), not build time.
ENTRYPOINT ["/eval/eval.sh"]
