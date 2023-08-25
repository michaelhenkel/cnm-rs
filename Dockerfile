FROM registry.default.svc.cluster.local:5000/debian:12
RUN apt update -y
RUN apt install -y openssl
COPY target/aarch64-unknown-linux-gnu/release/crpd-init /usr/bin
