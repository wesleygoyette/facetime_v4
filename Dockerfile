FROM rust:1-alpine AS builder

WORKDIR /app

COPY . .

RUN apk add --no-cache musl-dev

RUN cargo build --release --bin server

FROM alpine:latest

RUN apk add --no-cache bash

EXPOSE 8069
EXPOSE 8070

COPY --from=builder /app/target/release/server /usr/local/bin/server

ENTRYPOINT ["/usr/local/bin/server", "--udp", "fly-global-services" ]
