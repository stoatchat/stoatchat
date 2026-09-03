FROM caddy:2-alpine

# Baked-in default assets, shipped inside the image.
COPY default /srv/assets/default

# Self-hosters override this path with a volume mount to add or replace
# assets without rebuilding the image
COPY custom /srv/assets/custom

COPY Caddyfile /etc/caddy/Caddyfile

EXPOSE 14707

HEALTHCHECK --interval=30s --timeout=3s \
  CMD wget -qO- http://localhost:14707/health || exit 1