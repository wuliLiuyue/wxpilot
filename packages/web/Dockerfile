FROM node:20-bookworm-slim AS deps
WORKDIR /app
RUN corepack enable
WORKDIR /app/packages/web
COPY packages/web/package.json ./
RUN pnpm install --no-frozen-lockfile

FROM node:20-bookworm-slim AS build
WORKDIR /app
RUN corepack enable
COPY --from=deps /app/packages/web/node_modules ./packages/web/node_modules
COPY packages/web ./packages/web
WORKDIR /app/packages/web
RUN pnpm build

FROM nginx:1.27-alpine AS runtime
COPY --from=build /app/packages/web/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/packages/web/dist /usr/share/nginx/html
EXPOSE 80
HEALTHCHECK --interval=10s --timeout=3s --retries=3 CMD wget -qO- http://127.0.0.1/ >/dev/null || exit 1
CMD ["nginx", "-g", "daemon off;"]
