//! The message digests a PDF signature may use.

use der::oid::ObjectIdentifier;
use sha2::Digest;

use crate::oids;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hash {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl Hash {
    pub fn from_oid(oid: &ObjectIdentifier) -> Option<Self> {
        match *oid {
            oids::SHA1 => Some(Hash::Sha1),
            oids::SHA256 => Some(Hash::Sha256),
            oids::SHA384 => Some(Hash::Sha384),
            oids::SHA512 => Some(Hash::Sha512),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Hash::Sha1 => "SHA-1",
            Hash::Sha256 => "SHA-256",
            Hash::Sha384 => "SHA-384",
            Hash::Sha512 => "SHA-512",
        }
    }

    /// Digest of the concatenation of `parts`.
    pub fn digest(self, parts: &[&[u8]]) -> Vec<u8> {
        fn run<D: Digest>(parts: &[&[u8]]) -> Vec<u8> {
            let mut h = D::new();
            for p in parts {
                h.update(p);
            }
            h.finalize().to_vec()
        }
        match self {
            Hash::Sha1 => run::<sha1::Sha1>(parts),
            Hash::Sha256 => run::<sha2::Sha256>(parts),
            Hash::Sha384 => run::<sha2::Sha384>(parts),
            Hash::Sha512 => run::<sha2::Sha512>(parts),
        }
    }

    /// PKCS#1 v1.5 signature padding (DigestInfo prefix) for this digest.
    pub fn pkcs1v15(self) -> rsa::Pkcs1v15Sign {
        match self {
            Hash::Sha1 => rsa::Pkcs1v15Sign::new::<sha1::Sha1>(),
            Hash::Sha256 => rsa::Pkcs1v15Sign::new::<sha2::Sha256>(),
            Hash::Sha384 => rsa::Pkcs1v15Sign::new::<sha2::Sha384>(),
            Hash::Sha512 => rsa::Pkcs1v15Sign::new::<sha2::Sha512>(),
        }
    }
}
