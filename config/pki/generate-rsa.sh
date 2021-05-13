#!/bin/bash
# ------------------------------------------------------------------------------
# This script will generate a new private key and a Certificate Signing Request
# (CSR) using OpenSSL.
# This script is non-interactive. Instead it uses the variables set at the
# beginning of this script. Alternatively you can adapt this script easily
# to read the values differently as required.
# Developed and tested on Mac OS only, but should work on Linux too.
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.
#
# Created by Nick Zahn, Cloud Under Ltd - https://cloudunder.io
# ------------------------------------------------------------------------------

# Replace the following values:
SERVER=$1
COMMONNAME=$2 # Domain name, e.g. "cloudunder.io"
ORGANISATION="MIT" # e.g. company
LOCALITY="Cambridge" # e.g. city
STATE="Massachusetts" # state or province name
COUNTRY="US" # 2 letter code, e.g. "GB", "US", "DE"

# ------------------------------------------------------------------------------
# NO NEED TO EDIT ANYTHING BELOW THIS LINE (unless you want to)
# ------------------------------------------------------------------------------

PRIVATE_KEY_FILE="$SERVER-key.pem"
CERT_SIGN_REQUEST_FILE="$SERVER.csr"
CERT_FILE="$SERVER-cert.pem"
IDENTITY_FILE="$SERVER.p12"


cat <<EOF > .temp-openssl-config
[ req ]
default_bits           = 2048
distinguished_name     = req_distinguished_name
prompt                 = no
encrypt_key            = no
string_mask            = utf8only
req_extensions         = v3_req

[ req_distinguished_name ]
C                      = ${COUNTRY}
ST                     = ${STATE}
L                      = ${LOCALITY}
O                      = ${ORGANISATION}
CN                     = ${COMMONNAME}

[ v3_req ]
basicConstraints       = CA:FALSE
keyUsage               = nonRepudiation, digitalSignature, keyEncipherment
EOF

openssl genrsa -out ${PRIVATE_KEY_FILE} 2048
openssl req -new -config .temp-openssl-config -key ${PRIVATE_KEY_FILE} -out ${CERT_SIGN_REQUEST_FILE}
rm -f .temp-openssl-config
openssl x509 -req -in ${CERT_SIGN_REQUEST_FILE} -CA rootCA.pem -CAkey rootCA.key -CAcreateserial -out ${CERT_FILE} -days 3650
openssl pkcs12 -export -out ${IDENTITY_FILE} -inkey ${PRIVATE_KEY_FILE} -in ${CERT_FILE} -passout pass:solarwinds123

# Check
M_RSA=$(openssl rsa -noout -modulus -in ${PRIVATE_KEY_FILE})
M_REQ=$(openssl req -noout -modulus -in ${CERT_SIGN_REQUEST_FILE})
if [ "${M_RSA}" != "${M_REQ}" ]; then
	echo "Something went wrong. Private key and CSR files don't match."
	exit 1
fi

echo "Done. Files generated:"
echo ""
echo "  1. Private key:"
echo "     ${PRIVATE_KEY_FILE}"
echo "     > Keep this file safe. It will be required on the web server."
echo ""
echo "  2. Certificate Signing Request (CSR):"
echo "     ${CERT_SIGN_REQUEST_FILE}"
echo "     > Submit this file to the SSL certificate provider."
echo ""
echo "To see the decoded contents of the CSR file, run the following command:"
echo "  openssl req -verify -noout -text -in ${CERT_SIGN_REQUEST_FILE}"
