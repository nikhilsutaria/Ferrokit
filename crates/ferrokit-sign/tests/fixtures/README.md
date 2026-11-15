# ferrokit-sign test fixtures

**These keys are for tests only.** They were generated once with OpenSSL 3.5 in a throwaway
`alpine:3` container (see `generate.sh`), their passwords are public (below), and they must
never be used to sign anything real. All certificates are valid from 2026-01-01 to 2056-01-01 (UTC).

| File | Password | Contents |
| --- | --- | --- |
| `rsa.p12` | `ferrokit-rsa` | RSA-2048 key, self-signed certificate `CN=Ferrokit Test RSA` (AES-256 / PBKDF2, OpenSSL 3 default) |
| `rsa-legacy.p12` | `ferrokit-rsa` | Same key and certificate, legacy encryption (3DES + RC2, SHA-1 MAC) |
| `cert-only.p12` | `ferrokit-rsa` | The RSA certificate only, no private key |
| `ec.p12` | `ferrokit-ec` | ECDSA P-256 key, certificate `CN=Ferrokit Test EC Signer` (serial `01:F0:0D`) issued by `CN=Ferrokit Test CA`; the CA certificate is included |
| `p384.p12` | `ferrokit-p384` | ECDSA P-384 key, self-signed certificate `CN=Ferrokit Test P-384` |
| `ed25519.p12` | `ferrokit-ed` | Ed25519 key (an unsupported key type) |
| `encrypted.pdf` | user `test-user`, owner `test-owner` | One-page PDF encrypted by qpdf (AES-256, R6) |
| `pyhanko-pades-ec.pdf` | | Signed by pyHanko 0.38.1 with `ec.p12`: `/SubFilter /ETSI.CAdES.detached`, SHA-256, field `ApprovedBy` |
| `pyhanko-pkcs7-rsa.pdf` | | Signed by pyHanko 0.38.1 with `rsa.p12`: `/SubFilter /adbe.pkcs7.detached`, SHA-512, field `Reviewer` |
| `pyhanko-timestamps.pdf` | | pyHanko: a PAdES signature (`ec.p12`, field `Stamped`) carrying a signature timestamp, then an RFC 3161 document timestamp; pyHanko's offline dummy TSA used the `rsa.p12` key |

The pyHanko files check that `verify` accepts signatures made by an independent
implementation. They were made with `pyhanko_sign.py` (in a `python:3.12-slim` container with
`pip install pyhanko`) from a one-page PDF with a text field, produced by the crate's own tests.

To regenerate (this changes every key, so tests that pin serial numbers or subjects may need updating):

```sh
docker run --rm -v "$PWD/crates/ferrokit-sign/tests/fixtures:/out" -v "$PWD/crates/ferrokit-sign/tests/fixtures:/gen" alpine:3 sh /gen/generate.sh
```
