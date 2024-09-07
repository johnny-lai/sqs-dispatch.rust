FROM rust:1.81.0-slim-bullseye

WORKDIR /build

COPY Cargo.* .
COPY sqs-dispatch sqs-dispatch
COPY cmd cmd
COPY tests tests
