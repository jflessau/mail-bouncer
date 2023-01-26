FROM clux/muslrust:stable as build
COPY . .

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine
COPY --from=build /volume/target/x86_64-unknown-linux-musl/release/mail_bouncer /

CMD ["/mail_bouncer"]
