FROM rust:1.45 as builder-rust
WORKDIR /usr/src/fintrack
RUN apt-get update && apt-get -y install pkg-config libssl-dev
COPY . .
RUN cargo install --path .

FROM node:14 as builder-node
WORKDIR /app
COPY client/package.json .
RUN npm install
COPY client .
RUN npm run build

FROM ubuntu:focal
WORKDIR /app
RUN apt-get update && apt-get install -y libssl1.1 libcurl4
COPY --from=builder-rust /usr/local/cargo/bin/fintrack /usr/local/bin/fintrack
COPY --from=builder-node /app ./client
CMD ["fintrack"]
