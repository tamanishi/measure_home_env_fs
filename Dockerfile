FROM alpine:3.14 as build-stage

RUN apk add --no-cache rust
RUN apk add --no-cache cargo
RUN apk add --no-cache openssl-dev

WORKDIR /usr/src/measure_home_env_fs
COPY ./src/main.rs ./src/
COPY ./Cargo.toml ./

ENV PATH $PATH:/root/.cargo/bin
RUN cargo install --path .

FROM alpine:3.14

RUN apk add --no-cache libgcc
RUN apk add --no-cache tzdata
ENV TZ=Asia/Tokyo

COPY --from=build-stage /usr/src/measure_home_env_fs/target/release/measure_home_env_fs /usr/local/bin

# CMD ["measure_home_env_fs", "--dryrun"]
CMD ["measure_home_env_fs"]

