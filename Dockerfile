FROM rust:1.85.0

# Then set up the main project directory
WORKDIR /app

ARG ENV=test
ENV CONTAINER_ENV=$ENV

RUN apt-get update && apt-get install -y curl

RUN curl -L -o dotenvx.tar.gz "https://github.com/dotenvx/dotenvx/releases/latest/download/dotenvx-$(uname -s)-$(uname -m).tar.gz" \
    && tar -xzf dotenvx.tar.gz \
    && mv dotenvx /usr/local/bin \
    && rm dotenvx.tar.gz

COPY Cargo.toml .env .env.production .
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

EXPOSE 8080

ENV RUST_BACKTRACE=1

CMD if [ "$CONTAINER_ENV" = "prod" ]; then \
        dotenvx run -f .env.production -- ./target/release/utu_api; \
    else \
        dotenvx run -- ./target/debug/utu_api; \
    fi
