//! End-to-end tests: sign PDFs with the test certificates in `tests/fixtures` and check
//! them with `verify`. When `qpdf` is on PATH (or FERROKIT_QPDF), signed files are also
//! run through `qpdf --check` as an independent structural check.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use ferrokit_sign::{sign, verify, SignError, SignOptions, SignatureReport};
use lopdf::xref::XrefType;
use lopdf::{dictionary, Document, Object, Stream, StringFormat};

const RSA_PASSWORD: &str = "ferrokit-rsa";
const EC_PASSWORD: &str = "ferrokit-ec";
const P384_PASSWORD: &str = "ferrokit-p384";
const ED25519_PASSWORD: &str = "ferrokit-ed";

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()))
}

/// A two-page PDF with a classic xref table, as lopdf (and `qpdf --object-streams=disable`) write it.
fn sample_pdf() -> Vec<u8> {
    build_pdf(|_, _| {})
}

/// Builds a two-page PDF; `customise` may edit the document (catalog id, page ids) before saving.
fn build_pdf(customise: impl FnOnce(&mut Document, (lopdf::ObjectId, [lopdf::ObjectId; 2]))) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut page_ids = Vec::new();
    for i in 1..=2 {
        let content = format!("BT /F1 24 Tf 50 700 Td (Hello page {i}) Tj ET").into_bytes();
        let contents = doc.add_object(Stream::new(dictionary! {}, content));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), Object::Real(841.89)],
            "Contents" => contents,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        });
        page_ids.push(page);
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.iter().map(|&id| Object::Reference(id)).collect::<Vec<_>>(),
            "Count" => 2,
        }),
    );
    let info = doc.add_object(dictionary! {
        "Title" => Object::string_literal("Ferrokit signing sample"),
        "Producer" => Object::string_literal("ferrokit-sign tests"),
    });
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc.trailer.set("Info", info);
    let id = Object::String(b"0123456789abcdef".to_vec(), StringFormat::Hexadecimal);
    doc.trailer.set("ID", vec![id.clone(), id]);
    customise(&mut doc, (catalog, [page_ids[0], page_ids[1]]));
    // lopdf defaults to a cross-reference stream; normalised input has a classic table.
    doc.reference_table.cross_reference_type = XrefType::CrossReferenceTable;
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}

fn sign_rsa(pdf: &[u8]) -> Vec<u8> {
    sign(pdf, &fixture("rsa.p12"), RSA_PASSWORD, &SignOptions::default()).expect("RSA signing should work")
}

fn sign_ec(pdf: &[u8]) -> Vec<u8> {
    sign(pdf, &fixture("ec.p12"), EC_PASSWORD, &SignOptions::default()).expect("EC signing should work")
}

fn only_report(pdf: &[u8]) -> SignatureReport {
    let mut reports = verify(pdf).expect("verify should work");
    assert_eq!(reports.len(), 1, "expected exactly one signature: {reports:#?}");
    reports.remove(0)
}

fn assert_clean_valid(r: &SignatureReport) {
    assert!(r.signature_valid, "signature should be valid: {r:#?}");
    assert!(r.covers_whole_document, "signature should cover the whole file: {r:#?}");
    assert!(!r.modified_after_signing, "{r:#?}");
    assert_eq!(r.cert_valid_at_signing, Some(true), "{r:#?}");
    assert!(r.problems.is_empty(), "unexpected problems: {:?}", r.problems);
    assert_eq!(r.digest_algorithm, "SHA-256");
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn last_startxref(pdf: &[u8]) -> usize {
    let text = String::from_utf8_lossy(pdf);
    let at = text.rfind("startxref").unwrap();
    text[at + 9..].split_whitespace().next().unwrap().parse().unwrap()
}

/// Appends a genuine incremental update that changes the document info.
fn append_revision(pdf: &[u8]) -> Vec<u8> {
    let doc = Document::load_mem(pdf).unwrap();
    let size = doc.trailer.get(b"Size").unwrap().as_i64().unwrap();
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let prev = last_startxref(pdf);
    let mut out = pdf.to_vec();
    let offset = out.len();
    out.extend_from_slice(format!("{size} 0 obj\n<< /Producer (edited after signing) >>\nendobj\n").as_bytes());
    let xref = out.len();
    out.extend_from_slice(format!("xref\n{size} 1\n{offset:010} 00000 n\r\n").as_bytes());
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root {} {} R /Info {size} 0 R /Prev {prev} >>\nstartxref\n{xref}\n%%EOF\n",
            size + 1,
            root.0,
            root.1
        )
        .as_bytes(),
    );
    out
}

/// Runs `qpdf --check` when qpdf is available; any error or warning fails the test.
fn qpdf_check(pdf: &[u8]) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let qpdf = std::env::var("FERROKIT_QPDF").unwrap_or_else(|_| "qpdf".into());
    let path = std::env::temp_dir().join(format!(
        "ferrokit-sign-test-{}-{}.pdf",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::write(&path, pdf).unwrap();
    let result = Command::new(&qpdf).arg("--check").arg(&path).output();
    let _ = std::fs::remove_file(&path);
    match result {
        Err(_) => eprintln!("qpdf not found; skipping the structural check"),
        Ok(out) => assert!(
            out.status.success(),
            "qpdf --check failed ({}):\n{}{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

/// Text fields' or signature fields' names in the AcroForm, via lopdf.
fn acroform_field_names(pdf: &[u8]) -> Vec<String> {
    let doc = Document::load_mem(pdf).unwrap();
    let catalog = doc.catalog().unwrap();
    let acro = match catalog.get(b"AcroForm").unwrap() {
        Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
        Object::Dictionary(d) => d,
        other => panic!("unexpected AcroForm {other:?}"),
    };
    assert_eq!(acro.get(b"SigFlags").unwrap().as_i64().unwrap(), 3);
    let fields = match acro.get(b"Fields").unwrap() {
        Object::Reference(id) => doc.get_object(*id).unwrap().as_array().unwrap(),
        Object::Array(a) => a,
        other => panic!("unexpected Fields {other:?}"),
    };
    fields
        .iter()
        .map(|f| {
            let d = doc.get_dictionary(f.as_reference().unwrap()).unwrap();
            String::from_utf8(d.get(b"T").unwrap().as_str().unwrap().to_vec()).unwrap()
        })
        .collect()
}

#[test]
fn rsa_signature_verifies_and_covers_the_whole_document() {
    let original = sample_pdf();
    let signed = sign_rsa(&original);

    assert!(signed.starts_with(&original), "signing must only append (incremental update)");
    qpdf_check(&signed);

    let r = only_report(&signed);
    assert_clean_valid(&r);
    assert_eq!(r.field_name, "Signature1");
    assert_eq!(r.signer_name.as_deref(), Some("Ferrokit Test RSA"), "falls back to the certificate's CN");
    assert_eq!(r.signer_subject, "CN=Ferrokit Test RSA,O=Ferrokit Tests,C=NZ");
    assert_eq!(r.issuer, r.signer_subject);
    assert!(r.self_signed);
    assert_eq!(r.signature_algorithm, "RSA PKCS#1 v1.5 (2048-bit key)");
    assert_eq!(r.cert_not_before, "2026-01-01T00:00:00Z");
    assert_eq!(r.cert_not_after, "2056-01-01T00:00:00Z");
    assert!(!r.serial.is_empty());
    let time = r.signing_time.as_deref().expect("signing time");
    assert_eq!(time.len(), 20, "RFC 3339 UTC: {time}");
    assert!(time.ends_with('Z') && &time[4..5] == "-" && &time[10..11] == "T", "{time}");
    assert_eq!(r.reason, None);
    assert_eq!(r.location, None);

    // The structure other readers rely on.
    assert_eq!(acroform_field_names(&signed), vec!["Signature1"]);
    let doc = Document::load_mem(&signed).unwrap();
    let page1 = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
    let annots = page1.get(b"Annots").unwrap().as_array().unwrap();
    let widget = doc.get_dictionary(annots[0].as_reference().unwrap()).unwrap();
    assert_eq!(widget.get(b"FT").unwrap().as_name().unwrap(), b"Sig");
    assert_eq!(widget.get(b"Rect").unwrap().as_array().unwrap().len(), 4);
    let sig = doc.get_dictionary(widget.get(b"V").unwrap().as_reference().unwrap()).unwrap();
    assert_eq!(sig.get(b"Filter").unwrap().as_name().unwrap(), b"Adobe.PPKLite");
    assert_eq!(sig.get(b"SubFilter").unwrap().as_name().unwrap(), b"adbe.pkcs7.detached");
    assert!(sig.get(b"M").is_ok());
    assert!(doc.trailer.get(b"Info").unwrap().as_reference().is_ok(), "trailer keys are carried over");
    assert!(doc.trailer.get(b"ID").is_ok());
}

#[test]
fn ec_p256_signature_with_chain_verifies() {
    let signed = sign_ec(&sample_pdf());
    qpdf_check(&signed);
    let r = only_report(&signed);
    assert_clean_valid(&r);
    assert_eq!(r.signature_algorithm, "ECDSA P-256");
    assert_eq!(r.signer_name.as_deref(), Some("Ferrokit Test EC Signer"));
    assert_eq!(r.issuer, "CN=Ferrokit Test CA,O=Ferrokit Tests");
    assert_eq!(r.serial, "01:F0:0D");
    assert!(!r.self_signed);

    // The CA certificate from the p12 is embedded too: its name then appears four times in the
    // CMS (leaf issuer, signer id, CA subject, CA issuer) instead of two.
    let ca_name_hex: String = "Ferrokit Test CA".bytes().map(|b| format!("{b:02X}")).collect();
    let count = signed.windows(ca_name_hex.len()).filter(|w| *w == ca_name_hex.as_bytes()).count();
    assert!(count >= 4, "CA certificate should be included in the signature (found the name {count} times)");
}

#[test]
fn ec_p384_signature_verifies() {
    let signed = sign(&sample_pdf(), &fixture("p384.p12"), P384_PASSWORD, &SignOptions::default()).unwrap();
    let r = only_report(&signed);
    assert_clean_valid(&r);
    assert_eq!(r.signature_algorithm, "ECDSA P-384");
    assert!(r.self_signed);
}

#[test]
fn legacy_encrypted_p12_works() {
    let signed = sign(&sample_pdf(), &fixture("rsa-legacy.p12"), RSA_PASSWORD, &SignOptions::default()).unwrap();
    assert_clean_valid(&only_report(&signed));
}

#[test]
fn tampering_with_one_signed_byte_invalidates_the_signature() {
    let mut signed = sign_rsa(&sample_pdf());
    let at = find(&signed, b"Hello page 1").expect("page text is stored uncompressed");
    signed[at] = b'J';

    let r = only_report(&signed);
    assert!(!r.signature_valid, "{r:#?}");
    assert!(r.covers_whole_document, "the byte range itself is intact");
    assert!(r.problems.iter().any(|p| p.contains("changed")), "{:?}", r.problems);
    // Certificate details are still reported.
    assert_eq!(r.signer_name.as_deref(), Some("Ferrokit Test RSA"));
}

#[test]
fn appended_bytes_are_reported_as_modified_after_signing() {
    let signed = sign_rsa(&sample_pdf());

    // Trailing whitespace alone is not a modification.
    let mut padded = signed.clone();
    padded.extend_from_slice(b"\r\n\n");
    assert_clean_valid(&only_report(&padded));

    // Arbitrary trailing bytes are.
    let mut junk = signed.clone();
    junk.extend_from_slice(b"% appended later\n");
    let r = only_report(&junk);
    assert!(r.signature_valid && r.modified_after_signing && !r.covers_whole_document, "{r:#?}");

    // So is a genuine (short) incremental update.
    let edited = append_revision(&signed);
    qpdf_check(&edited);
    let r = only_report(&edited);
    assert!(r.signature_valid, "the signed revision itself is untouched: {r:#?}");
    assert!(r.modified_after_signing);
    assert!(!r.covers_whole_document);
}

#[test]
fn signing_twice_keeps_both_signatures_valid() {
    let once = sign_rsa(&sample_pdf());
    let twice = sign_ec(&once);
    assert!(twice.starts_with(&once));
    qpdf_check(&twice);

    let reports = verify(&twice).unwrap();
    assert_eq!(reports.len(), 2, "{reports:#?}");
    let (first, second) = (&reports[0], &reports[1]);
    assert_eq!(first.field_name, "Signature1");
    assert_eq!(first.signer_name.as_deref(), Some("Ferrokit Test RSA"));
    assert!(first.signature_valid, "{first:#?}");
    assert!(first.modified_after_signing, "the second signature was appended after the first");
    assert!(!first.covers_whole_document);

    assert_eq!(second.field_name, "Signature2");
    assert_eq!(second.signer_name.as_deref(), Some("Ferrokit Test EC Signer"));
    assert_clean_valid(second);
    assert_eq!(acroform_field_names(&twice), vec!["Signature1", "Signature2"]);

    // A third signature on top of an update made by someone else.
    let thrice = sign_rsa(&append_revision(&twice));
    qpdf_check(&thrice);
    let reports = verify(&thrice).unwrap();
    assert_eq!(reports.len(), 3);
    assert!(reports.iter().all(|r| r.signature_valid), "{reports:#?}");
    assert_eq!(reports[2].field_name, "Signature3");
    assert!(reports[2].covers_whole_document);
}

#[test]
fn signatures_made_by_another_tool_verify() {
    // Signed once with pyHanko (see tests/fixtures/README.md).
    let pades = fixture("pyhanko-pades-ec.pdf");
    let r = only_report(&pades);
    assert_clean_valid(&r);
    assert_eq!(r.field_name, "ApprovedBy");
    assert_eq!(r.reason.as_deref(), Some("Signed by pyHanko (PAdES)"));
    assert_eq!(r.location.as_deref(), Some("Elsewhere"));
    assert_eq!(r.signer_name.as_deref(), Some("Ferrokit Test EC Signer"));
    assert_eq!(r.signature_algorithm, "ECDSA P-256");
    // PAdES signatures carry no signing-time attribute, so the time comes from /M.
    assert_eq!(r.signing_time.as_deref(), Some("2026-11-13T09:09:21Z"));

    let r = only_report(&fixture("pyhanko-pkcs7-rsa.pdf"));
    assert!(r.signature_valid && r.covers_whole_document && r.problems.is_empty(), "{r:#?}");
    assert_eq!(r.field_name, "Reviewer");
    assert_eq!(r.digest_algorithm, "SHA-512");

    // Signing on top of someone else's signature keeps theirs valid.
    let both = sign_rsa(&pades);
    qpdf_check(&both);
    let reports = verify(&both).unwrap();
    assert_eq!(reports.len(), 2, "{reports:#?}");
    assert_eq!(reports[0].field_name, "ApprovedBy");
    assert!(reports[0].signature_valid && reports[0].modified_after_signing, "{:#?}", reports[0]);
    assert_eq!(reports[1].field_name, "Signature1");
    assert_clean_valid(&reports[1]);
}

#[test]
fn signature_and_document_timestamps_are_understood() {
    let reports = verify(&fixture("pyhanko-timestamps.pdf")).unwrap();
    assert_eq!(reports.len(), 2, "{reports:#?}");
    let (signature, timestamp) = (&reports[0], &reports[1]);

    // A signature with an unsigned (timestamp) attribute still verifies.
    assert_eq!(signature.field_name, "Stamped");
    assert!(signature.signature_valid, "{signature:#?}");
    assert!(signature.modified_after_signing, "the document timestamp came later");

    // The RFC 3161 document timestamp: imprint and TSA signature check out.
    assert!(timestamp.signature_valid, "{timestamp:#?}");
    assert!(timestamp.covers_whole_document);
    assert!(timestamp.problems.is_empty(), "{:?}", timestamp.problems);
    assert_eq!(timestamp.signer_name.as_deref(), Some("Ferrokit Test RSA"), "the TSA's certificate");
    assert!(timestamp.signing_time.as_deref().is_some_and(|t| t.starts_with("2026-11-13T")), "{timestamp:#?}");
    assert_eq!(timestamp.cert_valid_at_signing, Some(true));

    // Tampering with the timestamped bytes is caught through the message imprint.
    let mut tampered = fixture("pyhanko-timestamps.pdf");
    let at = find(&tampered, b"Hello page 1").unwrap();
    tampered[at] = b'J';
    let reports = verify(&tampered).unwrap();
    assert!(reports.iter().all(|r| !r.signature_valid), "{reports:#?}");
}

#[test]
fn certified_documents_that_forbid_changes_are_not_signed() {
    let pdf = build_pdf(|doc, (catalog, _)| {
        let sig = doc.add_object(dictionary! {
            "Type" => "Sig",
            "Reference" => vec![Object::Dictionary(dictionary! {
                "Type" => "SigRef",
                "TransformMethod" => "DocMDP",
                "TransformParams" => dictionary! { "Type" => "TransformParams", "P" => 1, "V" => "1.2" },
            })],
        });
        doc.get_dictionary_mut(catalog).unwrap().set("Perms", dictionary! { "DocMDP" => sig });
    });
    let err = sign(&pdf, &fixture("rsa.p12"), RSA_PASSWORD, &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::BadRequest(ref m) if m.contains("certified")), "{err:?}");
}

#[test]
fn existing_acroform_and_annotation_arrays_are_reused() {
    // Indirect AcroForm with a text field, and page 1's /Annots as an indirect array.
    let mut acro_id = (0, 0);
    let pdf = build_pdf(|doc, (catalog, pages)| {
        let text_field = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx",
            "T" => Object::string_literal("Signature1"),
            "Rect" => vec![50.into(), 50.into(), 250.into(), 80.into()],
            "P" => pages[0],
        });
        let annots = doc.add_object(vec![Object::Reference(text_field)]);
        let fields = doc.add_object(vec![Object::Reference(text_field)]);
        acro_id = doc.add_object(dictionary! { "Fields" => fields, "DA" => Object::string_literal("/Helv 0 Tf 0 g") });
        doc.get_dictionary_mut(pages[0]).unwrap().set("Annots", annots);
        doc.get_dictionary_mut(catalog).unwrap().set("AcroForm", acro_id);
    });
    let signed = sign_rsa(&pdf);
    qpdf_check(&signed);
    assert_clean_valid(&only_report(&signed));

    let doc = Document::load_mem(&signed).unwrap();
    assert_eq!(doc.catalog().unwrap().get(b"AcroForm").unwrap().as_reference().unwrap(), acro_id);
    let acro = doc.get_dictionary(acro_id).unwrap();
    assert!(acro.get(b"DA").is_ok(), "existing AcroForm entries are kept");
    // The existing "Signature1" text field forces a fresh name.
    assert_eq!(acroform_field_names(&signed), vec!["Signature1", "Signature2"]);
    let page1 = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
    let annots_id = page1.get(b"Annots").unwrap().as_reference().unwrap();
    assert_eq!(doc.get_object(annots_id).unwrap().as_array().unwrap().len(), 2);

    // Inline AcroForm dictionary in the catalog.
    let pdf = build_pdf(|doc, (catalog, _)| {
        doc.get_dictionary_mut(catalog)
            .unwrap()
            .set("AcroForm", dictionary! { "Fields" => Vec::<Object>::new(), "SigFlags" => 0 });
    });
    let signed = sign_rsa(&pdf);
    qpdf_check(&signed);
    assert_clean_valid(&only_report(&signed));
    assert_eq!(acroform_field_names(&signed), vec!["Signature1"]);
}

#[test]
fn name_reason_location_and_contact_round_trip() {
    let opts: SignOptions = serde_json::from_value(serde_json::json!({
        "name": "Zoë Ngata",
        "reason": "Approved (final) \\ signed",
        "location": "Te Whanganui-a-Tara, NZ",
        "contact": "zoe@example.com",
    }))
    .unwrap();
    let signed = sign(&sample_pdf(), &fixture("rsa.p12"), RSA_PASSWORD, &opts).unwrap();
    let r = only_report(&signed);
    assert_clean_valid(&r);
    assert_eq!(r.signer_name.as_deref(), Some("Zoë Ngata"));
    assert_eq!(r.reason.as_deref(), Some("Approved (final) \\ signed"));
    assert_eq!(r.location.as_deref(), Some("Te Whanganui-a-Tara, NZ"));
    let doc = Document::load_mem(&signed).unwrap();
    let contact = doc.objects.values().find_map(|o| match o {
        Object::Dictionary(d) => d.get(b"ContactInfo").ok().map(|c| c.as_str().unwrap().to_vec()),
        _ => None,
    });
    assert_eq!(contact.as_deref(), Some(b"zoe@example.com".as_slice()));

    // Missing options deserialize as None; blank ones are left out.
    let opts: SignOptions = serde_json::from_str(r#"{"reason": "   "}"#).unwrap();
    assert!(opts.name.is_none() && opts.location.is_none() && opts.contact.is_none());
    let signed = sign(&sample_pdf(), &fixture("rsa.p12"), RSA_PASSWORD, &opts).unwrap();
    assert_eq!(only_report(&signed).reason, None);
}

#[test]
fn report_serializes_as_camel_case_json() {
    let r = only_report(&sign_rsa(&sample_pdf()));
    let json = serde_json::to_value(&r).unwrap();
    for key in [
        "fieldName",
        "signerName",
        "signerSubject",
        "issuer",
        "serial",
        "signingTime",
        "reason",
        "location",
        "digestAlgorithm",
        "signatureAlgorithm",
        "signatureValid",
        "coversWholeDocument",
        "modifiedAfterSigning",
        "selfSigned",
        "certNotBefore",
        "certNotAfter",
        "certValidAtSigning",
        "problems",
    ] {
        assert!(json.get(key).is_some(), "missing {key} in {json}");
    }
    assert_eq!(json["signatureValid"], true);
}

#[test]
fn unsigned_pdf_has_no_signatures() {
    assert!(verify(&sample_pdf()).unwrap().is_empty());
    // An empty signature field (no /V) is not a signature either.
    let pdf = build_pdf(|doc, (catalog, pages)| {
        let empty = doc.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "T" => Object::string_literal("Sign here"),
            "Rect" => vec![0.into(), 0.into(), 0.into(), 0.into()], "P" => pages[0],
        });
        doc.get_dictionary_mut(catalog).unwrap().set("AcroForm", dictionary! { "Fields" => vec![Object::Reference(empty)] });
    });
    assert!(verify(&pdf).unwrap().is_empty());
}

#[test]
fn wrong_password_is_reported() {
    let err = sign(&sample_pdf(), &fixture("rsa.p12"), "not the password", &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::WrongPassword), "{err:?}");
    assert!(err.is_client_error());
    assert!(err.to_string().contains("password"));

    let err = sign(&sample_pdf(), &fixture("rsa-legacy.p12"), "nope", &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::WrongPassword), "{err:?}");
}

#[test]
fn encrypted_pdfs_are_rejected() {
    let encrypted = fixture("encrypted.pdf");
    let err = sign(&encrypted, &fixture("rsa.p12"), RSA_PASSWORD, &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::Encrypted), "{err:?}");
    assert!(err.is_client_error());
    assert!(matches!(verify(&encrypted).unwrap_err(), SignError::Encrypted));
}

#[test]
fn certificate_files_without_a_usable_key_are_rejected() {
    let err = sign(&sample_pdf(), &fixture("cert-only.p12"), RSA_PASSWORD, &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::BadRequest(ref m) if m.contains("private key")), "{err:?}");

    let err = sign(&sample_pdf(), &fixture("ed25519.p12"), ED25519_PASSWORD, &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::Unsupported(ref m) if m.contains("Ed25519")), "{err:?}");
    assert!(err.is_client_error());

    let err = sign(&sample_pdf(), b"definitely not a p12", "x", &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::BadRequest(_)), "{err:?}");
}

#[test]
fn unreadable_and_unsupported_pdfs_are_rejected() {
    let err = sign(b"not a pdf at all", &fixture("rsa.p12"), RSA_PASSWORD, &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::BadRequest(_)), "{err:?}");
    assert!(matches!(verify(b"not a pdf at all").unwrap_err(), SignError::BadRequest(_)));

    // A cross-reference stream (what qpdf normalisation removes) is refused with a clear message.
    let mut doc = Document::load_mem(&sample_pdf()).unwrap();
    doc.reference_table.cross_reference_type = XrefType::CrossReferenceStream;
    let mut modern = Vec::new();
    doc.save_to(&mut modern).unwrap();
    assert!(find(&modern, b"/XRef").is_some(), "test setup: expected an xref stream");
    let err = sign(&modern, &fixture("rsa.p12"), RSA_PASSWORD, &SignOptions::default()).unwrap_err();
    assert!(matches!(err, SignError::Unsupported(ref m) if m.contains("cross-reference")), "{err:?}");
    assert!(err.is_client_error());
}

#[test]
fn malformed_signatures_are_reported_not_fatal() {
    let signed = sign_rsa(&sample_pdf());
    let contents = find(&signed, b"/Contents <").unwrap() + b"/Contents <".len();

    // Garbage instead of the CMS blob.
    let mut garbage = signed.clone();
    garbage[contents..contents + 16].copy_from_slice(b"DEADBEEFDEADBEEF");
    let r = only_report(&garbage);
    assert!(!r.signature_valid);
    assert!(!r.problems.is_empty(), "{r:#?}");

    // The signature value itself altered (the last bytes of the CMS are the signature).
    let mut altered = signed.clone();
    let end = contents + find(&signed[contents..], b">").unwrap();
    let last_hex = contents + signed[contents..end].iter().rposition(|&b| b != b'0').unwrap();
    altered[last_hex] = if altered[last_hex] == b'1' { b'2' } else { b'1' };
    let r = only_report(&altered);
    assert!(!r.signature_valid, "{r:#?}");
    assert!(r.problems.iter().any(|p| p.contains("doesn't verify")), "{:?}", r.problems);

    // A byte range pointing outside the file (same width, so offsets stay put).
    let mut outside = signed.clone();
    let br = find(&signed, b"/ByteRange [").unwrap() + b"/ByteRange ".len();
    let fake = b"[0 1 2 999999999]";
    outside[br..br + fake.len()].copy_from_slice(fake);
    outside[br + fake.len()..br + 36].fill(b' ');
    let r = only_report(&outside);
    assert!(!r.signature_valid);
    assert!(r.problems.iter().any(|p| p.contains("outside the file")), "{:?}", r.problems);

    // A byte range whose gap is not the /Contents string.
    let mut shifted = signed.clone();
    let text = String::from_utf8_lossy(&signed[br..br + 36]).to_string();
    let nums: Vec<usize> =
        text.trim_matches(|c| c == '[' || c == ']' || c == ' ').split_whitespace().map(|n| n.parse().unwrap()).collect();
    let moved = format!("[0 {} {} {}]", nums[1] - 20, nums[2], nums[3]);
    shifted[br..br + 36].fill(b' ');
    shifted[br..br + moved.len()].copy_from_slice(moved.as_bytes());
    let r = only_report(&shifted);
    assert!(!r.signature_valid);
    assert!(!r.covers_whole_document);
    assert!(!r.problems.is_empty());
}
