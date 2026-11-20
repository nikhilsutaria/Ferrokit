# Security

Ferrokit handles other people's files (contracts, scans, family videos), so security problems matter
even on a home network. This page explains what Ferrokit protects against, what it deliberately
doesn't, and how to report a problem.

## Reporting a problem

Please don't open a public issue for a security problem. Report it privately through GitHub instead:

1. Go to [Report a vulnerability](https://github.com/nikhilsutaria/Ferrokit/security/advisories/new)
   (the repository's **Security** tab).
2. Describe the problem and submit.

Useful things to include:

- what an attacker could do, and from where (another device on the network, or a file someone opens)
- the steps to reproduce it, with a sample file or request if you have one
- the commit you tested and which add-ons were running

Ferrokit is maintained by one person in their spare time, so this is best effort: you should hear back
within 7 days, and for a confirmed problem, get a plan for the fix within 30 days. Please hold off
publishing details until a fix is out. If you'd like credit, you'll be named in the fix's notes.

## Supported versions

Only the latest release gets security fixes. If you pinned an older version in `docker-compose.yml`,
move to the latest one.

## What Ferrokit is built to resist

Ferrokit is meant to run on a home network for people who trust each other. Within that, it assumes any
uploaded file could be hostile.

- **Uploads are treated as hostile.** They have size limits, and an upload to an add-on that stalls
  is dropped. EPUBs are unpacked with a cap on their real unpacked size, so a ZIP bomb can't fill the
  disk. Media uploads must really be audio, video or
  subtitle files; playlists and anything else that points to other files are refused, and ffmpeg may
  only read the uploaded files.
- **Files don't linger.** Server tools work in RAM and delete files as soon as the result is sent.
  Background job results are deleted an hour after they finish. Browser tools never upload anything.
- **Containers are locked down.** Every container runs as a non-root user on a read-only filesystem,
  with `no-new-privileges` and its scratch space in RAM. The add-ons publish no ports; only Ferrokit
  can reach them.
- **The Media add-on has no internet access.** It sits on an internal Docker network shared only with
  Ferrokit.
- **Web page to PDF can't reach the add-on's internal services.** Everything Chromium loads goes
  through a proxy that refuses the add-on's own addresses (loopback and link-local).
- **Stuck work is stopped.** Work in the add-ons has a time limit, and a timed-out or cancelled job has
  its whole process tree killed.

## What's out of scope

Some things are true by design:

- **Anyone who can reach Ferrokit can use all of it.** There are no accounts, so anyone on the network
  can use every tool, and see, download or cancel any background job. Don't expose Ferrokit to the
  internet; use a VPN, or a reverse proxy that asks for a password.
- **Web page to PDF can fetch pages on your network.** That's so you can print pages from your own
  devices. Only the add-on's own addresses (loopback and link-local) are blocked.
- **Someone on your network can keep the server busy** with large uploads, within the size and job
  limits you set.

Also out of scope:

- bugs in the bundled programs (LibreOffice, Chromium, Ghostscript, Tesseract, OCRmyPDF, ffmpeg, qpdf)
  that Ferrokit doesn't make worse; please report those to the program's own project
- attacks that need control of the server or the Docker host

If you're not sure whether something counts, report it anyway.
