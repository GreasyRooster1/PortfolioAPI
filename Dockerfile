FROM rust:1.96-slim AS builder
LABEL authors="dwil"

COPY . .

RUN ["cargo","build","--release"]

FROM debian AS runner
COPY --from=builder /target/release/PortfolioAPI .
COPY .env .
EXPOSE 8080
CMD ["./PortfolioAPI"]
