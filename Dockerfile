FROM node:alpine AS web-stage

WORKDIR /build
COPY package.json ./
COPY package-lock.json ./
COPY web ./web

RUN npm install
RUN npm run build

FROM rust:alpine AS build-stage

RUN apk update
RUN apk add cmake make musl-dev g++ perl

WORKDIR /build
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY --from=web-stage /build/resources/assets/main.css ./resources/assets/main.css
COPY --from=web-stage /build/resources/assets/main.js ./resources/assets/main.js

RUN cargo build --release

# Build image from scratch
FROM scratch
LABEL org.opencontainers.image.source="https://github.com/diz-unimr/mv-dashboard"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"
LABEL org.opencontainers.image.description="Application to show the state of MV §64e cases"

COPY --from=build-stage /build/target/release/mv-dashboard .
USER 65532

CMD ["./mv-dashboard"]
