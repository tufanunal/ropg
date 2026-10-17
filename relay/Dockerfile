# Lightweight, fast Node.js 22 runtime for Rogp Relay Server
FROM node:22-alpine

# Set working directory inside container
WORKDIR /app

# Copy relay package files
COPY relay/package*.json ./relay/

# Set production environment
ENV NODE_ENV=production
ENV PORT=8080

# Copy application sources
COPY shared/ ./shared/
COPY viewer/ ./viewer/
COPY relay/ ./relay/

# Expose HTTP port
EXPOSE 8080

# Healthcheck against Relay stats endpoint
HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:8080/api/stats || exit 1

# Start the Relay Server
WORKDIR /app/relay
CMD ["node", "server.js"]
