#!/usr/bin/env bash

openssl genrsa -out private.pem 2048
openssl rsa -in private.pem -pubout -outform PEM -out public.pem

# jwk generated at https://russelldavies.github.io/jwk-creator/