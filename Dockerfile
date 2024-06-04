FROM rust

# update default packages
RUN apt-get update

# Add source
RUN mkdir /rust-smart-fhir
ADD Cargo.toml /rust-smart-fhir/Cargo.toml
ADD lib /rust-smart-fhir/lib
ADD resources /rust-smart-fhir/resources
ADD src /rust-smart-fhir/src

# set working directory
WORKDIR /rust-smart-fhir

# build
RUN cargo install --path .

# expose ports and set domain
EXPOSE 80
ENV FHIR_EXAMPLE_PORT 80
ENV FHIR_EXAMPLE_HOSTNAME "0.0.0.0"

ENTRYPOINT ["/rust-smart-fhir/target/release/rust-smart-fhir"]
