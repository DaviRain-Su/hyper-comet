# Optional PlatformExecutor image (gate-only; keys never baked in).
FROM node:22-bookworm-slim
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm install
COPY tsconfig.json tsconfig.build.json ./
COPY src ./src
RUN npm run build && npm prune --omit=dev
# Bake proof-forge-next + olean cache here for real gate jobs.
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENV PROOFSHIP_DEVICE_ID=platform-1
ENTRYPOINT ["/entrypoint.sh"]
