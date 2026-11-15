//! The signer: a private key plus certificate chain from a PKCS#12 file, and the CMS
//! (PKCS#7) SignedData it produces.

use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{
    CertificateSet, EncapsulatedContentInfo, SignedData, SignerIdentifier, SignerInfo, SignerInfos,
};
use der::asn1::{Null, OctetString, SetOfVec};
use der::oid::ObjectIdentifier;
use der::{Any, Decode, Encode, Sequence};
use p12_keystore::Pkcs12Archive;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, PrivateKeyInfoRef};
use rsa::traits::PublicKeyParts;
use sha2::{Digest, Sha256};
use p256::ecdsa::signature::hazmat::PrehashSigner;
use spki::AlgorithmIdentifierOwned;
use x509_cert::attr::Attribute;
use x509_cert::time::Time;
use x509_cert::Certificate;

use crate::digest::Hash;
use crate::error::{Result, SignError};
use crate::oids;

const NOT_A_P12: &str =
    "This file couldn't be read as a .p12/.pfx certificate. Check that you chose the right file.";
const NO_KEY: &str = "This certificate file has no private key, so it can't be used to sign. \
     Export the certificate again with its private key included (a .p12 or .pfx file).";
const NO_MATCHING_CERT: &str = "This certificate file has a private key but not the certificate that \
     belongs to it. Export the certificate again with its full chain included.";

enum Key {
    Rsa(Box<rsa::PrivateKey>),
    P256(p256::ecdsa::SigningKey),
    P384(p384::ecdsa::SigningKey),
}

/// A key that can sign, with its certificate and any other certificates from the file.
pub(crate) struct Signer {
    key: Key,
    cert: Certificate,
    chain: Vec<Certificate>,
}

/// ESSCertIDv2 (RFC 5035) with the default SHA-256 hash algorithm (omitted in DER).
#[derive(Sequence)]
struct EssCertIdV2 {
    cert_hash: OctetString,
}

/// SigningCertificateV2 (RFC 5035), without policies.
#[derive(Sequence)]
struct SigningCertificateV2 {
    certs: Vec<EssCertIdV2>,
}

impl Signer {
    /// Opens a .p12/.pfx file and picks the first private key that has a matching certificate.
    pub fn from_p12(p12: &[u8], password: &str) -> Result<Self> {
        let archive = Pkcs12Archive::from_pkcs12(p12, password).map_err(p12_error)?;
        if archive.keys.is_empty() {
            return Err(SignError::bad(NO_KEY));
        }
        let certs: Vec<Certificate> =
            archive.certs.iter().filter_map(|bag| Certificate::from_der(bag.cert.as_der()).ok()).collect();

        let mut error = SignError::bad(NO_MATCHING_CERT);
        for bag in &archive.keys {
            let key = match Key::from_pkcs8(bag.key.as_der()) {
                Ok(key) => key,
                Err(e) => {
                    error = e;
                    continue;
                }
            };
            if let Some(i) = certs.iter().position(|c| key.matches(c)) {
                let mut chain = certs.clone();
                let cert = chain.remove(i);
                return Ok(Signer { key, cert, chain });
            }
        }
        Err(error)
    }

    /// Upper bound on the size of the DER-encoded CMS blob, from a dry run with a
    /// maximum-length dummy signature (only the signature length can vary).
    pub fn cms_size_bound(&self, time: &der::DateTime) -> Result<usize> {
        Ok(self.build_cms(&[0u8; 32], time, true)?.len())
    }

    /// Detached CMS SignedData over a SHA-256 `digest` of the signed byte ranges.
    pub fn sign_digest(&self, digest: &[u8], time: &der::DateTime) -> Result<Vec<u8>> {
        self.build_cms(digest, time, false)
    }

    fn build_cms(&self, digest: &[u8], time: &der::DateTime, dry_run: bool) -> Result<Vec<u8>> {
        let sha256 = AlgorithmIdentifierOwned { oid: oids::SHA256, parameters: None };
        let tbs = self.cert.tbs_certificate();

        let ess = SigningCertificateV2 {
            certs: vec![EssCertIdV2 { cert_hash: OctetString::new(Sha256::digest(self.cert.to_der()?).to_vec())? }],
        };
        let mut signed_attrs = SetOfVec::new();
        signed_attrs.insert(attribute(oids::CONTENT_TYPE, Any::encode_from(&oids::DATA)?)?)?;
        signed_attrs.insert(attribute(oids::SIGNING_TIME, Any::encode_from(&Time::from(*time))?)?)?;
        signed_attrs.insert(attribute(oids::MESSAGE_DIGEST, Any::encode_from(&OctetString::new(digest.to_vec())?)?)?)?;
        signed_attrs.insert(attribute(oids::SIGNING_CERTIFICATE_V2, Any::encode_from(&ess)?)?)?;

        // The signature covers the DER encoding of the attributes as a SET OF.
        let signature = if dry_run {
            vec![0xff; self.key.max_signature_len()]
        } else {
            self.key.sign(&signed_attrs.to_der()?)?
        };

        let signer_info = SignerInfo {
            version: CmsVersion::V1,
            sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
                issuer: tbs.issuer().clone(),
                serial_number: tbs.serial_number().clone(),
            }),
            digest_alg: sha256.clone(),
            signed_attrs: Some(signed_attrs),
            signature_algorithm: self.key.signature_algorithm()?,
            signature: OctetString::new(signature)?,
            unsigned_attrs: None,
        };

        let mut certs = SetOfVec::new();
        let mut seen: Vec<Vec<u8>> = Vec::new();
        for cert in std::iter::once(&self.cert).chain(&self.chain) {
            let der = cert.to_der()?;
            if !seen.contains(&der) {
                seen.push(der);
                certs.insert(CertificateChoices::Certificate(cert.clone()))?;
            }
        }

        let mut digest_algorithms = SetOfVec::new();
        digest_algorithms.insert(sha256)?;
        let mut signer_infos = SetOfVec::new();
        signer_infos.insert(signer_info)?;

        let signed_data = SignedData {
            version: CmsVersion::V1,
            digest_algorithms,
            encap_content_info: EncapsulatedContentInfo { econtent_type: oids::DATA, econtent: None },
            certificates: Some(CertificateSet(certs)),
            crls: None,
            signer_infos: SignerInfos(signer_infos),
        };
        let content_info = ContentInfo { content_type: oids::SIGNED_DATA, content: Any::encode_from(&signed_data)? };
        Ok(content_info.to_der()?)
    }
}

fn attribute(oid: ObjectIdentifier, value: Any) -> Result<Attribute> {
    let mut values = SetOfVec::new();
    values.insert(value)?;
    Ok(Attribute { oid, values })
}

impl Key {
    fn from_pkcs8(der: &[u8]) -> Result<Self> {
        let info = PrivateKeyInfoRef::try_from(der).map_err(|_| SignError::bad(NOT_A_P12))?;
        let alg = info.algorithm.oid;
        let curve = info.algorithm.parameters_oid().ok();
        let broken = |_| SignError::bad("The private key in this certificate file is damaged and can't be used.");
        match (alg, curve) {
            (oids::RSA_ENCRYPTION, _) => Ok(Key::Rsa(Box::new(rsa::PrivateKey::from_pkcs8_der(der).map_err(broken)?))),
            (oids::EC_PUBLIC_KEY, Some(oids::SECP256R1)) => {
                Ok(Key::P256(p256::ecdsa::SigningKey::from_pkcs8_der(der).map_err(broken)?))
            }
            (oids::EC_PUBLIC_KEY, Some(oids::SECP384R1)) => {
                Ok(Key::P384(p384::ecdsa::SigningKey::from_pkcs8_der(der).map_err(broken)?))
            }
            _ => Err(SignError::unsupported(format!(
                "This certificate uses {}, which can't be used for signing here. \
                 Use a certificate with an RSA key or an ECDSA P-256 or P-384 key.",
                describe_key(&alg, curve.as_ref())
            ))),
        }
    }

    /// True when `cert` holds the public half of this key.
    fn matches(&self, cert: &Certificate) -> bool {
        let Ok(spki) = cert.tbs_certificate().subject_public_key_info().to_der() else { return false };
        match self {
            Key::Rsa(k) => rsa::PublicKey::from_public_key_der(&spki).is_ok_and(|p| p == k.to_public_key()),
            Key::P256(k) => p256::ecdsa::VerifyingKey::from_public_key_der(&spki).is_ok_and(|p| &p == k.verifying_key()),
            Key::P384(k) => p384::ecdsa::VerifyingKey::from_public_key_der(&spki).is_ok_and(|p| &p == k.verifying_key()),
        }
    }

    fn max_signature_len(&self) -> usize {
        match self {
            Key::Rsa(k) => k.size(),
            // DER SEQUENCE of two INTEGERs, each at most field size + 1 sign byte.
            Key::P256(_) => 2 + 2 * (2 + 33),
            Key::P384(_) => 2 + 2 * (2 + 49),
        }
    }

    fn signature_algorithm(&self) -> Result<AlgorithmIdentifierOwned> {
        Ok(match self {
            Key::Rsa(_) => {
                AlgorithmIdentifierOwned { oid: oids::SHA256_WITH_RSA, parameters: Some(Any::encode_from(&Null)?) }
            }
            Key::P256(_) | Key::P384(_) => AlgorithmIdentifierOwned { oid: oids::ECDSA_WITH_SHA256, parameters: None },
        })
    }

    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let hash = Sha256::digest(msg);
        let failed = |e: &dyn std::fmt::Display| SignError::engine(format!("the signature couldn't be computed ({e})"));
        match self {
            Key::Rsa(k) => k.sign(Hash::Sha256.pkcs1v15(), &hash).map_err(|e| failed(&e)),
            Key::P256(k) => {
                let sig: p256::ecdsa::DerSignature = k.sign_prehash(&hash).map_err(|e| failed(&e))?;
                Ok(sig.as_bytes().to_vec())
            }
            Key::P384(k) => {
                let sig: p384::ecdsa::DerSignature = k.sign_prehash(&hash).map_err(|e| failed(&e))?;
                Ok(sig.as_bytes().to_vec())
            }
        }
    }
}

fn describe_key(alg: &ObjectIdentifier, curve: Option<&ObjectIdentifier>) -> String {
    match alg.to_string().as_str() {
        "1.3.101.112" => "an Ed25519 key".into(),
        "1.3.101.113" => "an Ed448 key".into(),
        "1.2.840.113549.1.1.10" => "an RSA-PSS key".into(),
        "1.2.840.10040.4.1" => "a DSA key".into(),
        "1.2.840.10045.2.1" => match curve.map(|c| c.to_string()).as_deref() {
            Some("1.3.132.0.10") => "an elliptic-curve key on the secp256k1 curve".into(),
            Some("1.3.132.0.35") => "an elliptic-curve key on the P-521 curve".into(),
            Some(c) if c.starts_with("1.3.36.3.3.2.8.1.1.") => "a Brainpool elliptic-curve key".into(),
            _ => "an elliptic-curve key on an unsupported curve".into(),
        },
        other => format!("an unsupported key type ({other})"),
    }
}

fn p12_error(e: p12_keystore::error::Error) -> SignError {
    use p12_keystore::error::Error as E;
    match e {
        E::MacError(_) | E::UnpadError | E::Pkcs5Error(_) => SignError::WrongPassword,
        E::UnsupportedEncryptionScheme | E::UnsupportedMacAlgorithm => SignError::unsupported(
            "This certificate file is protected with an encryption method that isn't supported. \
             Export it again with AES-256 encryption and try again.",
        ),
        _ => SignError::bad(NOT_A_P12),
    }
}
