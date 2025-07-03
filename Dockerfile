FROM rust:1.85.0

# Then set up the main project directory
WORKDIR /app

ARG ENV=test
ENV CONTAINER_ENV=$ENV

RUN apt-get update && apt-get install -y curl

# Install dotenvx
RUN curl -sfS https://dotenvx.sh/install.sh | sh

COPY Cargo.toml .env .env.production .env.keys .
COPY src ./src
# Copy the private dependency first at the root level
# todo: remove when made public
COPY ./utu_bridge_deposit_address ./utu_bridge_deposit_address
COPY ./utu_bridge_types ./utu_bridge_types

RUN if [ "$CONTAINER_ENV" = "prod" ]; then \
        cargo build --release; \
    else \
        cargo build; \
    fi

EXPOSE 80

ENV RUST_BACKTRACE=1

CMD if [ "$CONTAINER_ENV" = "prod" ]; then \
        dotenvx run -f .env.production -- ./target/release/utu_api; \
    else \
        dotenvx run -- ./target/debug/utu_api; \
    fi
