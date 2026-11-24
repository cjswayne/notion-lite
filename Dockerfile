FROM rust:1-slim AS build
WORKDIR /app
COPY . .
RUN cargo build --release -p notion-lite --bin notion-lite

FROM gcr.io/distroless/cc-debian12
COPY --from=build /app/target/release/notion-lite /notion-lite
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["/notion-lite"]
