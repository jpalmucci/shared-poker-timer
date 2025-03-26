#!/bin/zsh
set -e

# TODO - update the certs from the letsencrypt directory
docker push jpalmucci/pokertimer:latest

scp docker-compose.yaml root@$DROPLET:

ssh root@$DROPLET <<EOF
set -e
docker-compose down
docker-compose pull
docker-compose up -d --remove-orphans
EOF
