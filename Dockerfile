FROM debian:12
RUN apt update -y
RUN apt install -y openssl
COPY target/release/crpd-init /usr/bin
