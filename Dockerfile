FROM alpine:edge
MAINTAINER Alexey Smirnov <chemistmail@gmail.com>

RUN apk update && apk upgrade && \
    apk add --update rsync

COPY ./target/x86_64-unknown-linux-musl/release/rsync-resource /opt/resource/rsync-resource 
RUN ln /opt/resource/rsync-resource /opt/resource/in && ln /opt/resource/rsync-resource /opt/resource/check && ln /opt/resource/rsync-resource /opt/resource/out && chmod +x /opt/resource/rsync-resource
