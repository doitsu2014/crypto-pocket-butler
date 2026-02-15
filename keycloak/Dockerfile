FROM alpine:3.21

RUN apk add --no-cache curl bash jq openssl

COPY init-keycloak.sh /init-keycloak.sh
RUN chmod +x /init-keycloak.sh

CMD ["/init-keycloak.sh"]
