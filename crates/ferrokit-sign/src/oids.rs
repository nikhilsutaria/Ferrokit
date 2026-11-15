//! Object identifiers used by CMS signatures in PDFs.

use der::oid::ObjectIdentifier as Oid;

// CMS content types and attributes (RFC 5652, RFC 5035, RFC 3161).
pub const SIGNED_DATA: Oid = Oid::new_unwrap("1.2.840.113549.1.7.2");
pub const DATA: Oid = Oid::new_unwrap("1.2.840.113549.1.7.1");
pub const CONTENT_TYPE: Oid = Oid::new_unwrap("1.2.840.113549.1.9.3");
pub const MESSAGE_DIGEST: Oid = Oid::new_unwrap("1.2.840.113549.1.9.4");
pub const SIGNING_TIME: Oid = Oid::new_unwrap("1.2.840.113549.1.9.5");
pub const SIGNING_CERTIFICATE_V2: Oid = Oid::new_unwrap("1.2.840.113549.1.9.16.2.47");

// Digests.
pub const SHA1: Oid = Oid::new_unwrap("1.3.14.3.2.26");
pub const SHA256: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.2.1");
pub const SHA384: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.2.2");
pub const SHA512: Oid = Oid::new_unwrap("2.16.840.1.101.3.4.2.3");

// RSA.
pub const RSA_ENCRYPTION: Oid = Oid::new_unwrap("1.2.840.113549.1.1.1");
pub const SHA1_WITH_RSA: Oid = Oid::new_unwrap("1.2.840.113549.1.1.5");
pub const RSASSA_PSS: Oid = Oid::new_unwrap("1.2.840.113549.1.1.10");
pub const SHA256_WITH_RSA: Oid = Oid::new_unwrap("1.2.840.113549.1.1.11");
pub const SHA384_WITH_RSA: Oid = Oid::new_unwrap("1.2.840.113549.1.1.12");
pub const SHA512_WITH_RSA: Oid = Oid::new_unwrap("1.2.840.113549.1.1.13");

// Elliptic curves.
pub const EC_PUBLIC_KEY: Oid = Oid::new_unwrap("1.2.840.10045.2.1");
pub const ECDSA_WITH_SHA1: Oid = Oid::new_unwrap("1.2.840.10045.4.1");
pub const ECDSA_WITH_SHA256: Oid = Oid::new_unwrap("1.2.840.10045.4.3.2");
pub const ECDSA_WITH_SHA384: Oid = Oid::new_unwrap("1.2.840.10045.4.3.3");
pub const ECDSA_WITH_SHA512: Oid = Oid::new_unwrap("1.2.840.10045.4.3.4");
pub const SECP256R1: Oid = Oid::new_unwrap("1.2.840.10045.3.1.7");
pub const SECP384R1: Oid = Oid::new_unwrap("1.3.132.0.34");
