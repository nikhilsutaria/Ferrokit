//! Checking the signatures in a PDF.
//!
//! For each signature field the byte ranges and `/Contents` are taken straight from the
//! raw file, the covered bytes are hashed, and the CMS SignedData is checked: the
//! message-digest attribute must match, and the signature over the signed attributes
//! must verify with the signer certificate's public key. Trust chains are not checked.

use std::collections::HashSet;

use der::{Decode, Encode};
use lopdf::{Dictionary, Document, Object, ObjectId};
use p256::ecdsa::signature::hazmat::PrehashVerifier;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use x509_cert::ext::pkix::SubjectKeyIdentifier;
use x509_cert::name::Name;
use x509_cert::Certificate;

use crate::ber::{self, Tlv};
use crate::digest::Hash;
use crate::error::{Result, SignError, UNREADABLE_PDF};
use crate::{fields, oids, pdf, SignatureReport};

const CHANGED: &str =
    "The document has been changed since this signature was applied: the signed content no longer matches.";
const DAMAGED_CMS: &str = "The signature data is damaged or isn't a PKCS#7 signature, so it can't be checked.";
const BAD_MATHS: &str =
    "The signature doesn't verify with the signer's certificate: the signature or the certificate has been altered.";
const SHA1_WEAK: &str = "This signature uses SHA-1, which is no longer considered secure.";

pub(crate) fn verify(bytes: &[u8]) -> Result<Vec<SignatureReport>> {
    let doc = match pdf::load(bytes) {
        Ok(doc) => doc,
        Err(_) if pdf::mentions_encrypt(bytes) => return Err(SignError::Encrypted),
        Err(_) => return Err(SignError::bad(UNREADABLE_PDF)),
    };
    if doc.trailer.has(b"Encrypt") {
        return Err(SignError::Encrypted);
    }

    // Signature fields reachable from the AcroForm, then any stray ones elsewhere.
    let mut candidates: Vec<(String, &Object)> = Vec::new();
    let mut seen_values: HashSet<ObjectId> = HashSet::new();
    let mut reachable: HashSet<ObjectId> = HashSet::new();
    // A signature value shared by several field nodes is reported once.
    let mut first_sighting = |value: &Object| pdf::as_ref_id(value).is_none_or(|id| seen_values.insert(id));
    for field in fields::all_fields(&doc) {
        if let Some(id) = field.id {
            reachable.insert(id);
        }
        if field.ft != Some(b"Sig".as_slice()) {
            continue;
        }
        // No /V: an empty signature field, nothing signed yet.
        let Some(value) = field.dict.get(b"V").ok().filter(|v| !matches!(v, Object::Null)) else { continue };
        if first_sighting(value) {
            candidates.push((field.name, value));
        }
    }
    for (id, obj) in &doc.objects {
        let Object::Dictionary(dict) = obj else { continue };
        if reachable.contains(id) || pdf::get_name(&doc, dict, b"FT") != Some(b"Sig".as_slice()) {
            continue;
        }
        let Some(value) = dict.get(b"V").ok().filter(|v| !matches!(v, Object::Null)) else { continue };
        if first_sighting(value) {
            candidates.push((pdf::get_text(&doc, dict, b"T").unwrap_or_default(), value));
        }
    }

    let mut reports: Vec<(usize, SignatureReport)> = candidates
        .into_iter()
        .map(|(name, value)| {
            let name = if name.is_empty() { "Unnamed signature".to_string() } else { name };
            match pdf::resolve_dict(&doc, value) {
                Some(sig) => check_signature(bytes, &doc, name, sig),
                None => {
                    let mut report = SignatureReport::unchecked(name);
                    report.problems.push("The signature data is missing or damaged, so it can't be checked.".into());
                    (usize::MAX, report)
                }
            }
        })
        .collect();
    reports.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.field_name.cmp(&b.1.field_name)));
    Ok(reports.into_iter().map(|(_, r)| r).collect())
}

/// Facts gathered while checking, used once the main checks are done.
#[derive(Default)]
struct Facts {
    signing_unix: Option<i64>,
    validity_unix: Option<(i64, i64)>,
}

/// Checks one signature dictionary. Returns the report and where its byte range ends
/// (for ordering signatures by revision).
fn check_signature(bytes: &[u8], doc: &Document, field_name: String, sig: &Dictionary) -> (usize, SignatureReport) {
    let mut report = SignatureReport::unchecked(field_name);
    report.signer_name = pdf::get_text(doc, sig, b"Name");
    report.reason = pdf::get_text(doc, sig, b"Reason");
    report.location = pdf::get_text(doc, sig, b"Location");

    let mut facts = Facts::default();
    let mut end = usize::MAX;
    if let Err(problem) = check_inner(bytes, doc, sig, &mut report, &mut facts, &mut end) {
        report.signature_valid = false;
        report.problems.push(problem);
    }

    // Fall back to the claimed signing time (/M) when the CMS doesn't carry one.
    if report.signing_time.is_none() {
        if let Some((text, unix)) = pdf::get_text(doc, sig, b"M").and_then(|m| pdf::parse_pdf_date(&m)) {
            report.signing_time = Some(text);
            facts.signing_unix = Some(unix);
        }
    }
    if let (Some(t), Some((not_before, not_after))) = (facts.signing_unix, facts.validity_unix) {
        let valid = not_before <= t && t <= not_after;
        report.cert_valid_at_signing = Some(valid);
        if !valid {
            report.problems.push(
                "The signer's certificate wasn't valid when the document was signed (it had expired or wasn't valid yet)."
                    .into(),
            );
        }
    }
    (end, report)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    /// adbe.pkcs7.detached / ETSI.CAdES.detached: the CMS signs the byte ranges directly.
    Detached,
    /// adbe.pkcs7.sha1: the CMS signs an embedded SHA-1 digest of the byte ranges.
    Sha1Embedded,
    /// ETSI.RFC3161: a document timestamp token whose message imprint covers the byte ranges.
    Timestamp,
}

fn check_inner(
    bytes: &[u8],
    doc: &Document,
    sig: &Dictionary,
    report: &mut SignatureReport,
    facts: &mut Facts,
    end_out: &mut usize,
) -> std::result::Result<(), String> {
    // 1. Byte range and coverage.
    let [a, b, c, d] = byte_range(doc, sig).ok_or("The signature has no valid byte range, so it can't be checked.")?;
    let outside = || "The signature's byte range points outside the file, so it can't be checked.".to_string();
    let first_end = a.checked_add(b).ok_or_else(outside)?;
    let second_end = c.checked_add(d).ok_or_else(outside)?;
    if first_end > c || second_end > bytes.len() {
        return Err(outside());
    }
    *end_out = second_end;
    report.modified_after_signing = !only_trailer_noise(&bytes[second_end..]);
    let gap = &bytes[first_end..c];
    let gap_is_contents = gap.len() >= 2
        && gap[0] == b'<'
        && gap[gap.len() - 1] == b'>'
        && gap[1..gap.len() - 1].iter().all(|&ch| ch.is_ascii_hexdigit() || pdf::is_ws(ch));
    report.covers_whole_document = a == 0 && gap_is_contents && !report.modified_after_signing;
    if a != 0 {
        report.problems.push("The signature doesn't cover the start of the file.".into());
    }

    // 2. The signature value: straight from the raw bytes, cross-checked with /Contents.
    let declared = match sig.get(b"Contents").ok().and_then(|o| pdf::resolve(doc, o)) {
        Some(Object::String(s, _)) => Some(s.clone()),
        _ => None,
    };
    let contents = if gap_is_contents {
        let raw = pdf::decode_hex(&gap[1..gap.len() - 1]).ok_or(DAMAGED_CMS)?;
        if declared.as_ref().is_some_and(|dec| *dec != raw) {
            return Err(
                "The signature's byte range doesn't line up with its signature value, so it can't be trusted.".into()
            );
        }
        raw
    } else {
        report.problems.push(
            "The signature's byte range leaves out more than the signature value, so parts of the file aren't protected."
                .into(),
        );
        declared.ok_or("The signature has no signature value, so it can't be checked.")?
    };

    let format = match pdf::get_name(doc, sig, b"SubFilter").unwrap_or(b"") {
        b"adbe.pkcs7.detached" | b"ETSI.CAdES.detached" | b"" => Format::Detached,
        b"adbe.pkcs7.sha1" => Format::Sha1Embedded,
        b"ETSI.RFC3161" => Format::Timestamp,
        b"adbe.x509.rsa_sha1" => {
            return Err("This signature uses the old adbe.x509.rsa_sha1 format, which can't be checked here.".into())
        }
        other => {
            report.problems.push(format!(
                "Unknown signature format ({}); it was checked as a detached PKCS#7 signature.",
                String::from_utf8_lossy(other)
            ));
            Format::Detached
        }
    };

    // 3. The CMS structure and the signer's certificate.
    let cms = parse_cms(&contents)?;
    let cert = find_signer(&cms.certs, &cms.sid)
        .ok_or("The signer's certificate isn't included in the signature, so it can't be checked.")?;
    describe_certificate(report, facts, cert);

    // 4. The digest of the signed content.
    let hash = Hash::from_oid(&cms.digest_alg)
        .ok_or_else(|| format!("Unsupported digest algorithm ({}), so the signature can't be checked.", cms.digest_alg))?;
    report.digest_algorithm = hash.name().into();
    if hash == Hash::Sha1 {
        report.problems.push(SHA1_WEAK.into());
    }
    let ranges = [&bytes[a..first_end], &bytes[c..second_end]];
    let content: Vec<&[u8]> = match format {
        Format::Detached => ranges.to_vec(),
        Format::Sha1Embedded => {
            let embedded = cms.econtent.as_deref().ok_or(DAMAGED_CMS)?;
            if embedded != Hash::Sha1.digest(&ranges).as_slice() {
                return Err(CHANGED.into());
            }
            vec![embedded]
        }
        Format::Timestamp => {
            let token = cms.econtent.as_deref().ok_or(DAMAGED_CMS)?;
            let (time, unix) = check_timestamp(token, &ranges)?;
            report.signing_time = Some(time);
            facts.signing_unix = Some(unix);
            vec![token]
        }
    };
    let message_digest = hash.digest(&content);

    // 5. Signed attributes: the message digest must match; the signature covers them.
    let signed_attrs = match &cms.signed_attrs {
        Some(attrs) => {
            if attrs.indefinite {
                return Err("The signature's signed attributes aren't DER-encoded, so it can't be checked.".into());
            }
            let mut claimed_digest = None;
            for attr in attrs.children().ok_or(DAMAGED_CMS)? {
                let Some(parts) = attr.children() else { continue };
                let oid = parts.first().and_then(Tlv::oid);
                let first = parts.get(1).and_then(Tlv::children).and_then(|values| values.into_iter().next());
                let (Some(oid), Some(first)) = (oid, first) else { continue };
                if oid == oids::MESSAGE_DIGEST {
                    claimed_digest = first.octets();
                } else if oid == oids::SIGNING_TIME && report.signing_time.is_none() {
                    if let Some((text, unix)) = parse_asn1_time(&first) {
                        report.signing_time = Some(text);
                        facts.signing_unix = Some(unix);
                    }
                }
            }
            let claimed = claimed_digest
                .ok_or("The signature has no document fingerprint (message digest), so it can't be checked.")?;
            if claimed != message_digest {
                return Err(CHANGED.into());
            }
            // Signed over the DER SET OF encoding: same bytes, universal SET tag.
            let mut der = attrs.raw.to_vec();
            der[0] = ber::SET;
            Some(der)
        }
        None => None,
    };

    // 6. The signature itself.
    let sig_hash = match cms.sig_alg {
        oids::SHA1_WITH_RSA | oids::ECDSA_WITH_SHA1 => Hash::Sha1,
        oids::SHA256_WITH_RSA | oids::ECDSA_WITH_SHA256 => Hash::Sha256,
        oids::SHA384_WITH_RSA | oids::ECDSA_WITH_SHA384 => Hash::Sha384,
        oids::SHA512_WITH_RSA | oids::ECDSA_WITH_SHA512 => Hash::Sha512,
        oids::RSA_ENCRYPTION | oids::EC_PUBLIC_KEY => hash,
        oids::RSASSA_PSS => {
            report.signature_algorithm = "RSA-PSS".into();
            return Err("RSA-PSS signatures aren't supported yet, so this signature can't be checked.".into());
        }
        other => return Err(format!("Unsupported signature algorithm ({other}), so the signature can't be checked.")),
    };
    let prehash = match &signed_attrs {
        Some(der) => sig_hash.digest(&[der]),
        None => sig_hash.digest(&content),
    };
    let rsa_alg = matches!(
        cms.sig_alg,
        oids::RSA_ENCRYPTION | oids::SHA1_WITH_RSA | oids::SHA256_WITH_RSA | oids::SHA384_WITH_RSA | oids::SHA512_WITH_RSA
    );
    let spki = cert
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .map_err(|_| "The signer's certificate is damaged.".to_string())?;
    let mismatch = || "The signature's algorithm doesn't match the signer's key, so it can't be checked.".to_string();
    let verified = if let Ok(key) = rsa::PublicKey::from_public_key_der(&spki) {
        report.signature_algorithm = format!("RSA PKCS#1 v1.5 ({}-bit key)", key.size() * 8);
        if !rsa_alg {
            return Err(mismatch());
        }
        key.verify(sig_hash.pkcs1v15(), &prehash, &cms.signature).is_ok()
    } else if let Ok(key) = p256::ecdsa::VerifyingKey::from_public_key_der(&spki) {
        report.signature_algorithm = "ECDSA P-256".into();
        if rsa_alg {
            return Err(mismatch());
        }
        let sig = ecdsa_signature(&cms.signature, 32).and_then(|rs| p256::ecdsa::Signature::from_slice(&rs).ok());
        sig.is_some_and(|s| key.verify_prehash(&prehash, &s).is_ok())
    } else if let Ok(key) = p384::ecdsa::VerifyingKey::from_public_key_der(&spki) {
        report.signature_algorithm = "ECDSA P-384".into();
        if rsa_alg {
            return Err(mismatch());
        }
        let sig = ecdsa_signature(&cms.signature, 48).and_then(|rs| p384::ecdsa::Signature::from_slice(&rs).ok());
        sig.is_some_and(|s| key.verify_prehash(&prehash, &s).is_ok())
    } else {
        let alg = cert.tbs_certificate().subject_public_key_info().algorithm.oid;
        return Err(format!(
            "The signer's key type isn't supported ({}), so the signature can't be checked. \
             Supported keys are RSA and ECDSA P-256/P-384.",
            key_name(&alg)
        ));
    };
    if !verified {
        return Err(BAD_MATHS.into());
    }
    report.signature_valid = true;
    Ok(())
}

/// The four integers of `/ByteRange` (two ranges); anything else is rejected.
fn byte_range(doc: &Document, sig: &Dictionary) -> Option<[usize; 4]> {
    let Object::Array(items) = pdf::resolve(doc, sig.get(b"ByteRange").ok()?)? else { return None };
    if items.len() != 4 {
        return None;
    }
    let mut out = [0usize; 4];
    for (slot, item) in out.iter_mut().zip(items) {
        match pdf::resolve(doc, item)? {
            Object::Integer(n) if *n >= 0 => *slot = usize::try_from(*n).ok()?,
            _ => return None,
        }
    }
    Some(out)
}

/// True if `tail` is only whitespace, optionally around one `%%EOF` marker.
fn only_trailer_noise(tail: &[u8]) -> bool {
    let skip_ws = |s: &[u8]| -> usize { s.iter().take_while(|&&b| pdf::is_ws(b)).count() };
    let mut rest = &tail[skip_ws(tail)..];
    if let Some(after) = rest.strip_prefix(b"%%EOF") {
        rest = &after[skip_ws(after)..];
    }
    rest.is_empty()
}

/// The parts of a CMS SignedData that matter for checking its first signer.
struct Cms<'a> {
    econtent: Option<Vec<u8>>,
    certs: Vec<Certificate>,
    sid: Tlv<'a>,
    digest_alg: der::oid::ObjectIdentifier,
    signed_attrs: Option<Tlv<'a>>,
    sig_alg: der::oid::ObjectIdentifier,
    signature: Vec<u8>,
}

fn parse_cms(contents: &[u8]) -> std::result::Result<Cms<'_>, String> {
    // Trailing zero padding after the DER blob is normal; `ber::parse` ignores it.
    let (content_info, _) = ber::parse(contents).ok_or(DAMAGED_CMS)?;
    let parts = content_info.children().ok_or(DAMAGED_CMS)?;
    if parts.first().and_then(Tlv::oid) != Some(oids::SIGNED_DATA) {
        return Err("The signature data isn't a PKCS#7 signed-data structure, so it can't be checked.".into());
    }
    parse_signed_data(&parts).ok_or_else(|| DAMAGED_CMS.to_string())
}

fn parse_signed_data<'a>(content_info: &[Tlv<'a>]) -> Option<Cms<'a>> {
    let explicit = content_info.get(1).filter(|t| t.tag == ber::CTX0)?;
    let signed_data = explicit.children()?.into_iter().next().filter(|t| t.tag == ber::SEQUENCE)?;
    let mut items = signed_data.children()?.into_iter();
    let _version = items.next().filter(|t| t.tag == ber::INTEGER)?;
    let _digest_algorithms = items.next().filter(|t| t.tag == ber::SET)?;
    let encap = items.next().filter(|t| t.tag == ber::SEQUENCE)?;
    let econtent = match encap.children()?.get(1) {
        Some(wrapper) if wrapper.tag == ber::CTX0 => Some(wrapper.children()?.first()?.octets()?),
        _ => None,
    };
    let mut certs = Vec::new();
    let mut signer_infos = None;
    for item in items {
        match item.tag {
            ber::CTX0 => {
                for cert in item.children()? {
                    if cert.tag == ber::SEQUENCE {
                        if let Ok(cert) = Certificate::from_der(cert.raw) {
                            certs.push(cert);
                        }
                    }
                }
            }
            ber::SET => signer_infos = Some(item),
            _ => {}
        }
    }
    let signer = signer_infos?.children()?.into_iter().next().filter(|t| t.tag == ber::SEQUENCE)?;
    let mut fields = signer.children()?.into_iter();
    let _version = fields.next()?;
    let sid = fields.next()?;
    let digest_alg = fields.next()?.children()?.first()?.oid()?;
    let mut next = fields.next()?;
    let signed_attrs = if next.tag == ber::CTX0 {
        let attrs = next;
        next = fields.next()?;
        Some(attrs)
    } else {
        None
    };
    let sig_alg = next.children()?.first()?.oid()?;
    let signature = fields.next().filter(|t| t.tag & 0x1f == ber::OCTET_STRING)?.octets()?;
    Some(Cms { econtent, certs, sid, digest_alg, signed_attrs, sig_alg, signature })
}

/// The certificate named by the SignerIdentifier (issuer + serial, or subject key id).
fn find_signer<'c>(certs: &'c [Certificate], sid: &Tlv) -> Option<&'c Certificate> {
    let strip = |b: &[u8]| -> Vec<u8> { b.iter().skip_while(|&&x| x == 0).copied().collect() };
    match sid.tag {
        ber::SEQUENCE => {
            let parts = sid.children()?;
            let (issuer, serial) = (parts.first()?, parts.get(1).filter(|t| t.tag == ber::INTEGER)?);
            let issuer_name = Name::from_der(issuer.raw).ok();
            certs.iter().find(|cert| {
                let tbs = cert.tbs_certificate();
                let same_issuer = tbs.issuer().to_der().is_ok_and(|der| der == issuer.raw)
                    || issuer_name.as_ref().is_some_and(|n| n == tbs.issuer());
                same_issuer && strip(tbs.serial_number().as_bytes()) == strip(serial.value)
            })
        }
        ber::CTX0_PRIMITIVE => certs.iter().find(|cert| {
            matches!(
                cert.tbs_certificate().get_extension::<SubjectKeyIdentifier>(),
                Ok(Some((_, ski))) if ski.0.as_bytes() == sid.value
            )
        }),
        _ => None,
    }
}

fn describe_certificate(report: &mut SignatureReport, facts: &mut Facts, cert: &Certificate) {
    let tbs = cert.tbs_certificate();
    report.signer_subject = tbs.subject().to_string();
    report.issuer = tbs.issuer().to_string();
    report.serial = tbs.serial_number().to_string();
    if report.signer_name.is_none() {
        report.signer_name = tbs.subject().common_name().ok().flatten().map(|cn| cn.value().into_owned());
    }
    report.self_signed = tbs.subject() == tbs.issuer();
    let validity = tbs.validity();
    report.cert_not_before = pdf::rfc3339(&validity.not_before.to_date_time());
    report.cert_not_after = pdf::rfc3339(&validity.not_after.to_date_time());
    facts.validity_unix = Some((
        validity.not_before.to_unix_duration().as_secs() as i64,
        validity.not_after.to_unix_duration().as_secs() as i64,
    ));
}

/// Checks an RFC 3161 TSTInfo's message imprint against the byte ranges and returns its time.
fn check_timestamp(tst_info: &[u8], ranges: &[&[u8]]) -> std::result::Result<(String, i64), String> {
    let damaged = || "The document timestamp is damaged, so it can't be checked.".to_string();
    let (tst, _) = ber::parse(tst_info).ok_or_else(damaged)?;
    let parts = tst.children().ok_or_else(damaged)?;
    let imprint = parts.get(2).and_then(Tlv::children).ok_or_else(damaged)?;
    let alg = imprint.first().and_then(Tlv::children).and_then(|a| a.first().and_then(Tlv::oid)).ok_or_else(damaged)?;
    let hashed = imprint.get(1).and_then(Tlv::octets).ok_or_else(damaged)?;
    let hash = Hash::from_oid(&alg).ok_or_else(|| format!("Unsupported timestamp digest algorithm ({alg})."))?;
    if hash.digest(ranges) != hashed {
        return Err(CHANGED.into());
    }
    parts.get(4).and_then(parse_asn1_time).ok_or_else(damaged)
}

/// UTCTime / GeneralizedTime (fractional seconds tolerated) to RFC 3339 + Unix seconds.
fn parse_asn1_time(t: &Tlv) -> Option<(String, i64)> {
    let text = std::str::from_utf8(t.value).ok()?;
    match t.tag {
        0x17 => {
            let yy: u32 = text.get(0..2)?.parse().ok()?;
            let century = if yy >= 50 { "19" } else { "20" };
            pdf::parse_pdf_date(&format!("{century}{text}"))
        }
        0x18 => pdf::parse_pdf_date(text),
        _ => None,
    }
}

/// `r || s`, each left-padded to `size` bytes, from a DER/BER ECDSA-Sig-Value.
fn ecdsa_signature(der: &[u8], size: usize) -> Option<Vec<u8>> {
    let (seq, _) = ber::parse(der)?;
    let parts = seq.children()?;
    if seq.tag != ber::SEQUENCE || parts.len() != 2 {
        return None;
    }
    let mut out = Vec::with_capacity(2 * size);
    for int in &parts {
        if int.tag != ber::INTEGER {
            return None;
        }
        let digits: Vec<u8> = int.value.iter().skip_while(|&&b| b == 0).copied().collect();
        if digits.len() > size {
            return None;
        }
        out.resize(out.len() + size - digits.len(), 0);
        out.extend_from_slice(&digits);
    }
    Some(out)
}

fn key_name(oid: &der::oid::ObjectIdentifier) -> String {
    match oid.to_string().as_str() {
        "1.3.101.112" => "Ed25519".into(),
        "1.3.101.113" => "Ed448".into(),
        "1.2.840.113549.1.1.10" => "RSA-PSS".into(),
        "1.2.840.10040.4.1" => "DSA".into(),
        "1.2.840.10045.2.1" => "elliptic curve other than P-256/P-384".into(),
        "1.2.840.113549.1.1.1" => "RSA key of an unsupported size".into(),
        other => other.to_string(),
    }
}
