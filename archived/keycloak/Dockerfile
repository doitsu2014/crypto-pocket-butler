FROM ubuntu:22.04

RUN apt-get update && apt-get install -y curl bash jq openssl ca-certificates && rm -rf /var/lib/apt/lists/*

COPY init-keycloak.sh /init-keycloak.sh
RUN chmod +x /init-keycloak.sh

CMD ["/init-keycloak.sh"]
