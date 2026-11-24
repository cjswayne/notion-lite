FROM rust:1-slim AS build
WORKDIR /app
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=build /app/target/release/notion-lite /notion-lite
ENV PORT=8080
EXPOSE 8080
CMD ["/notion-lite"]
