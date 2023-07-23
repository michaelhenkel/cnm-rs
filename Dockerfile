FROM debian:12
RUN echo hello
COPY target/release/crpd-init /usr/bin