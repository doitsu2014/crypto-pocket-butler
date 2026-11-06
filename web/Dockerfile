# Build from the repository root:
#   docker build -f web/Dockerfile -t web .
FROM node:24-alpine AS builder
WORKDIR /app
RUN corepack enable

COPY web/package.json web/yarn.lock web/.yarnrc.yml ./
RUN yarn install --immutable

COPY web ./
RUN yarn build

FROM nginxinc/nginx-unprivileged:1.27-alpine AS runtime
COPY --from=builder /app/dist /usr/share/nginx/html
COPY web/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1
