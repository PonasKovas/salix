#!/bin/sh

mkdir certs
openssl ecparam -name prime256v1 -genkey -out certs/server.key
openssl req -new -x509 -key certs/server.key -out certs/server.crt -days 3650 -subj "/CN=unused"
