#!/bin/sh

mkdir -p certs
openssl genpkey -algorithm ED25519 -out certs/server.key
openssl req -new -x509 -key certs/server.key -out certs/server.crt -days 3650 -subj "/CN=unused"
