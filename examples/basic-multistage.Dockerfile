FROM rust:latest as builder
RUN USER=root cargo new --bin basic

WORKDIR /basic

COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs ./target/release/deps/basic*
ADD . ./
RUN cargo build --release

FROM rust:latest
ARG APP=/code
ARG APP_USER=root

RUN groupadd $APP_USER && useradd -g $APP_USER $APP_USER && mkdir -p $APP
COPY --from=builder /basic//target/release/basic $APP/basic

USER $USER
WORKDIR $APP
    