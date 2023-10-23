#!/usr/bin/env bash

CURVE="secp521r1" # or "secp384r1" (ES384) or "secp521r1" (ES512)


openssl ecparam -name $CURVE -genkey -noout -out private.pem
openssl ec -in private.pem -pubout -out public.pem

# TODO: jwk
