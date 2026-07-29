FROM rust:1.80-slim AS builder
LABEL authors="dwil"

COPY . .

RUN cargo build --release

FROM debian
COPY --from=builder /target/release/PortfolioAPI .
EXPOSE 8080
CMD ["PortfolioAPI"]
