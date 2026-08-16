# syntax=docker/dockerfile:1
# GaussMeridian WebUI — multi-stage Next.js 16 build.
# Build:  docker build -t gauss-meridian-webui .
# Run:    docker run -p 3000:3000 -e NEXT_PUBLIC_API_BASE_URL=http://host.docker.internal:8000 gauss-meridian-webui
# The API base URL is read server-side by the /api/gaussmeridian proxy at request time,
# so the runtime env var is what matters; the build ARG only sets a sane default.

FROM node:26-alpine AS deps
WORKDIR /app
# Pin the exact pnpm the lockfile (v9) was written with — independent of corepack.
RUN npm install -g pnpm@10.7.1
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

FROM node:26-alpine AS build
WORKDIR /app
RUN npm install -g pnpm@10.7.1
COPY --from=deps /app/node_modules ./node_modules
COPY . .
ARG NEXT_PUBLIC_API_BASE_URL=http://gaussmeridian:8000
ENV NEXT_PUBLIC_API_BASE_URL=$NEXT_PUBLIC_API_BASE_URL
ENV NEXT_TELEMETRY_DISABLED=1
RUN pnpm build

FROM node:26-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0
RUN addgroup -S nodejs && adduser -S nextjs -G nodejs
COPY --from=build /app/public ./public
COPY --from=build --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=build --chown=nextjs:nodejs /app/.next/static ./.next/static
USER nextjs
EXPOSE 3000
CMD ["node", "server.js"]
