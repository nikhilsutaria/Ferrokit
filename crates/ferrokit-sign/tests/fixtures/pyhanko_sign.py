from pyhanko.sign import signers, fields
from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter

def sign(p12, password, out, field, subfilter, reason, md='sha256'):
    signer = signers.SimpleSigner.load_pkcs12(p12, passphrase=password)
    with open('/dump/unsigned.pdf', 'rb') as inf:
        w = IncrementalPdfFileWriter(inf)
        meta = signers.PdfSignatureMetadata(field_name=field, reason=reason, location='Elsewhere',
                                            subfilter=subfilter, md_algorithm=md)
        with open(out, 'wb') as outf:
            signers.sign_pdf(w, meta, signer=signer, output=outf)

sign('/fx/ec.p12', b'ferrokit-ec', '/out/pyhanko-pades-ec.pdf', 'ApprovedBy',
     fields.SigSeedSubFilter.PADES, 'Signed by pyHanko (PAdES)')
sign('/fx/rsa.p12', b'ferrokit-rsa', '/out/pyhanko-pkcs7-rsa.pdf', 'Reviewer',
     fields.SigSeedSubFilter.ADOBE_PKCS7_DETACHED, 'Signed by pyHanko (PKCS#7)', md='sha512')
print('done')

# A PAdES signature with a signature timestamp (unsigned attribute), then an RFC 3161
# document timestamp on top. pyHanko's offline DummyTimeStamper acts as the TSA, using
# the test RSA key.
import io
from pyhanko.sign.timestamps.dummy_client import DummyTimeStamper
tsa = signers.SimpleSigner.load_pkcs12('/fx/rsa.p12', passphrase=b'ferrokit-rsa')
stamper = DummyTimeStamper(tsa_cert=tsa.signing_cert, tsa_key=tsa.signing_key)
signer = signers.SimpleSigner.load_pkcs12('/fx/ec.p12', passphrase=b'ferrokit-ec')
with open('/dump/unsigned.pdf', 'rb') as inf:
    buf = io.BytesIO()
    meta = signers.PdfSignatureMetadata(field_name='Stamped', subfilter=fields.SigSeedSubFilter.PADES)
    signers.sign_pdf(IncrementalPdfFileWriter(inf), meta, signer=signer, timestamper=stamper, output=buf)
    buf.seek(0)
    with open('/out/pyhanko-timestamps.pdf', 'wb') as outf:
        signers.PdfTimeStamper(stamper).timestamp_pdf(IncrementalPdfFileWriter(buf), md_algorithm='sha384', output=outf)
print('timestamps done')
