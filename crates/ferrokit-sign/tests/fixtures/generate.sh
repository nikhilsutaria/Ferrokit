#!/bin/sh
# Generates the TEST-ONLY certificates and PDFs used by ferrokit-sign's tests.
set -eu
apk add -q --no-cache openssl qpdf >/dev/null
openssl version
W=/work; O=/out
mkdir -p $W; cd $W
NB=20260101000000Z; NA=20560101000000Z

# 1. RSA-2048, self-signed.
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -keyout rsa.key -out rsa.crt \
  -not_before $NB -not_after $NA \
  -subj "/C=NZ/O=Ferrokit Tests/CN=Ferrokit Test RSA" \
  -addext "keyUsage=critical,digitalSignature,nonRepudiation"
openssl pkcs12 -export -inkey rsa.key -in rsa.crt -name "Ferrokit Test RSA" -out $O/rsa.p12 -passout pass:ferrokit-rsa
# Same key, legacy encryption (3DES + RC2, SHA-1 MAC), as older Windows/OpenSSL exports produce.
openssl pkcs12 -export -legacy -inkey rsa.key -in rsa.crt -out $O/rsa-legacy.p12 -passout pass:ferrokit-rsa
# Certificate only, no private key.
openssl pkcs12 -export -nokeys -in rsa.crt -out $O/cert-only.p12 -passout pass:ferrokit-rsa

# 2. ECDSA P-256 signer issued by a P-256 test CA; the p12 includes the CA certificate.
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -sha256 -keyout ca.key -out ca.crt \
  -not_before $NB -not_after $NA \
  -subj "/O=Ferrokit Tests/CN=Ferrokit Test CA" \
  -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -new -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -keyout ec.key -out ec.csr \
  -subj "/O=Ferrokit Tests/CN=Ferrokit Test EC Signer"
printf "keyUsage=critical,digitalSignature,nonRepudiation\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid\n" > ec.ext
openssl x509 -req -in ec.csr -CA ca.crt -CAkey ca.key -set_serial 0x1F00D -sha256 \
  -not_before $NB -not_after $NA -extfile ec.ext -out ec.crt
openssl pkcs12 -export -inkey ec.key -in ec.crt -certfile ca.crt -name "Ferrokit Test EC Signer" -out $O/ec.p12 -passout pass:ferrokit-ec

# 3. ECDSA P-384, self-signed.
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-384 -nodes -sha384 -keyout p384.key -out p384.crt \
  -not_before $NB -not_after $NA -subj "/O=Ferrokit Tests/CN=Ferrokit Test P-384"
openssl pkcs12 -export -inkey p384.key -in p384.crt -out $O/p384.p12 -passout pass:ferrokit-p384

# 4. Ed25519 (unsupported key type).
openssl req -x509 -newkey ed25519 -nodes -keyout ed.key -out ed.crt \
  -not_before $NB -not_after $NA -subj "/O=Ferrokit Tests/CN=Ferrokit Test Ed25519"
openssl pkcs12 -export -inkey ed.key -in ed.crt -out $O/ed25519.p12 -passout pass:ferrokit-ed

# 5. A one-page PDF, password protected with qpdf (AES-256).
cat > plain.pdf <<'PDF'
%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R >> endobj
4 0 obj << /Length 37 >> stream
BT /F1 12 Tf 20 150 Td (Locked) Tj ET
endstream endobj
trailer << /Root 1 0 R /Size 5 >>
%%EOF
PDF
qpdf plain.pdf fixed.pdf || [ $? -eq 3 ]
qpdf --encrypt test-user test-owner 256 -- fixed.pdf $O/encrypted.pdf
qpdf --show-encryption --password=test-user $O/encrypted.pdf | head -3
ls -la $O
