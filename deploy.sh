#!/bin/zsh
set -e

docker push jpalmucci/pokertimer:latest

scp docker-compose.yaml root@$DROPLET:

ssh root@$DROPLET <<EOF
set -e
cp /etc/letsencrypt/live/pokertimer.palmucci.net/privkey.pem certs/tls-key.pem
cp /etc/letsencrypt/live/pokertimer.palmucci.net/fullchain.pem certs/tls-cert.pem
docker-compose down
docker-compose pull
docker-compose up -d --remove-orphans
EOF
