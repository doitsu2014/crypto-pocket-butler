# Build from the repository root:
#   docker build -f web/Dockerfile -t web .
#
# The image serves the customer SPA at / and the support console at /support.
# Both bundles are built here so they share one immutable static artefact and
# one health-checked Swarm service.
FROM node:24-alpine AS builder
WORKDIR /app
RUN corepack enable

COPY web/package.json web/yarn.lock web/.yarnrc.yml ./web/
COPY support-console/package.json support-console/yarn.lock support-console/.yarnrc.yml ./support-console/
RUN cd web && yarn install --immutable
RUN cd support-console && yarn install --immutable

COPY web ./web
COPY support-console ./support-console

# Build-time configuration is baked into the static bundle. The gateway URL
# defaults to same-origin; the Supabase URL and anon key are public client
# configuration, never secrets (secrets live in Supabase Vault).
ARG VITE_GATEWAY_URL=/v1
ARG VITE_SUPABASE_URL=
ARG VITE_SUPABASE_ANON_KEY=
ENV VITE_GATEWAY_URL=$VITE_GATEWAY_URL
ENV VITE_SUPABASE_URL=$VITE_SUPABASE_URL
ENV VITE_SUPABASE_ANON_KEY=$VITE_SUPABASE_ANON_KEY

# Build order matters: the customer build empties web/dist, then the support
# build writes web/dist/support (support-console/vite.config.ts sets outDir).
RUN cd web && yarn build
RUN cd support-console && yarn build

FROM nginxinc/nginx-unprivileged:1.27-alpine AS runtime
COPY --from=builder /app/web/dist /usr/share/nginx/html
COPY web/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080 8081
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1
