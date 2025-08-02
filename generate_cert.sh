#!/bin/sh

rm -rf certs
mkdir -p certs

openssl genpkey -algorithm ED25519 -out certs/ca.key
openssl req -new -x509 -key certs/ca.key -out certs/ca.crt -days 3650 -subj "/CN=salix_ca" -addext "basicConstraints = critical, CA:TRUE"

openssl genpkey -algorithm ED25519 -out certs/server.key
openssl req -new -key certs/server.key -out certs/server.csr -subj "/CN=salix"
openssl x509 -req -in certs/server.csr -CA certs/ca.crt -CAkey certs/ca.key -out certs/server.crt -CAcreateserial -days 365 -extfile <(printf "subjectAltName=DNS:salix")

rm certs/server.csr
