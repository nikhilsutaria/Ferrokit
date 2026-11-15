// The whole Ferrokit catalog. A tool is "ready" when `src/lib/tools/<id>.svelte` exists;
// everything else shows as coming soon, so this file doubles as the roadmap.

import type { Component } from 'svelte';

export type Runtime = 'browser' | 'server' | 'addon';
export type Addon = 'office' | 'ocr' | 'media' | 'ai';

export interface Category {
  id: string;
  name: string;
  shelf: 'PDF' | 'Developer' | 'Everyday' | 'Media';
}

export interface Tool {
  id: string;
  name: string;
  category: string;
  description: string;
  keywords: string[];
  runtime: Runtime;
  addon?: Addon;
  ready: boolean;
}

export const categories: Category[] = [
  { id: 'pdf-organise', name: 'Organise PDF pages', shelf: 'PDF' },
  { id: 'pdf-edit', name: 'Edit PDFs', shelf: 'PDF' },
  { id: 'pdf-security', name: 'PDF security', shelf: 'PDF' },
  { id: 'pdf-optimise', name: 'Optimise and repair', shelf: 'PDF' },
  { id: 'pdf-convert', name: 'Convert PDFs', shelf: 'PDF' },
  { id: 'dev-crypto', name: 'Crypto and tokens', shelf: 'Developer' },
  { id: 'dev-convert', name: 'Converters', shelf: 'Developer' },
  { id: 'dev-web', name: 'Web', shelf: 'Developer' },
  { id: 'dev-development', name: 'Development', shelf: 'Developer' },
  { id: 'dev-network', name: 'Network', shelf: 'Developer' },
  { id: 'text', name: 'Text and maths', shelf: 'Everyday' },
  { id: 'files', name: 'Files and data', shelf: 'Everyday' },
  { id: 'image', name: 'Images', shelf: 'Media' },
  { id: 'media', name: 'Video and audio', shelf: 'Media' },
  { id: 'ai', name: 'Local AI', shelf: 'Media' },
];

const modules = import.meta.glob<{ default: Component }>('./tools/*.svelte');

export function loadTool(id: string): (() => Promise<{ default: Component }>) | undefined {
  return modules[`./tools/${id}.svelte`];
}

type Row = [id: string, name: string, description: string, keywords: string, runtime?: Runtime | Addon];

function add(category: string, rows: Row[]): Tool[] {
  return rows.map(([id, name, description, keywords, rt = 'browser']) => {
    const addon = rt === 'office' || rt === 'ocr' || rt === 'media' || rt === 'ai' ? rt : undefined;
    return {
      id,
      name,
      category,
      description,
      keywords: keywords.split(' ').filter(Boolean),
      runtime: addon ? 'addon' : (rt as Runtime),
      addon,
      ready: `./tools/${id}.svelte` in modules,
    };
  });
}

export const tools: Tool[] = [
  ...add('pdf-organise', [
    ['merge-pdf', 'Merge PDFs', 'Combine several PDFs into one, in the order you choose.', 'combine join append concatenate', 'server'],
    ['split-pdf', 'Split PDF', 'Break a PDF into several files: every N pages, by ranges, or one per page.', 'separate divide break burst', 'server'],
    ['page-editor', 'Page editor', 'See every page, then drag to reorder, rotate or delete.', 'organise organize reorder arrange thumbnails sort', 'server'],
    ['rotate-pdf', 'Rotate pages', 'Turn some or all pages by 90, 180 or 270 degrees.', 'turn orientation landscape portrait', 'server'],
    ['remove-pages', 'Remove pages', 'Delete the pages you list and keep the rest.', 'delete drop', 'server'],
    ['extract-pages', 'Extract pages', 'Make a new PDF from just the pages you list.', 'select keep pick subset', 'server'],
    ['insert-blank', 'Insert blank pages', 'Add empty pages before or after chosen pages.', 'blank empty add', 'server'],
    ['remove-blank', 'Remove blank pages', 'Find and drop pages with nothing on them.', 'blank empty clean scan', 'server'],
    ['n-up', 'Multi-page layout', 'Print several pages per sheet: 2-up, 4-up or a folded booklet.', 'booklet imposition nup 2-up 4-up', 'server'],
    ['single-page', 'Single large page', 'Stitch all pages into one tall page.', 'stitch long page', 'server'],
    ['split-advanced', 'Split by bookmarks or size', 'Split at each chapter bookmark or when a file reaches a size.', 'chapters outline size', 'server'],
    ['overlay-pdf', 'Overlay PDFs', 'Lay one PDF on top of or behind another, like letterhead.', 'letterhead underlay background', 'server'],
    ['alternate-merge', 'Merge scanned fronts and backs', 'Interleave a fronts scan with a reversed backs scan.', 'duplex scan interleave', 'server'],
  ]),
  ...add('pdf-edit', [
    ['page-numbers', 'Add page numbers', 'Number pages in any corner, with your own wording like "Page 3 of 10".', 'numbering footer header', 'server'],
    ['watermark-pdf', 'Text watermark', 'Stamp text such as DRAFT or CONFIDENTIAL across pages.', 'stamp draft confidential', 'server'],
    ['image-stamp', 'Image watermark', 'Place a logo or image on every page.', 'logo stamp', 'server'],
    ['crop-pdf', 'Crop pages', 'Trim the margins off pages.', 'trim margins', 'server'],
    ['scale-pdf', 'Change page size', 'Fit pages to A4, Letter or a custom size.', 'resize a4 letter scale', 'server'],
    ['pdf-metadata', 'Edit metadata', 'Change or clear the title, author, subject and keywords.', 'properties title author info exif', 'server'],
    ['remove-annotations', 'Remove annotations', 'Strip comments, highlights and markup.', 'comments highlights markup', 'server'],
    ['flatten-pdf', 'Flatten', 'Turn form fields and annotations into plain page content.', 'forms burn', 'server'],
    ['fill-form', 'Fill a form', 'Type into a PDF form and save it.', 'acroform fields'],
    ['add-image-pdf', 'Add an image', 'Place a picture anywhere on a page.', 'picture insert'],
    ['annotate-pdf', 'Annotate', 'Add text, drawings and highlights.', 'draw highlight comment'],
    ['pdf-colours', 'Grayscale', 'Turn a colour PDF black and white; text stays selectable.', 'grayscale greyscale black white monochrome', 'server'],
    ['compare-pdf', 'Compare PDFs', 'Show the text differences between two PDFs.', 'diff difference versions'],
    ['edit-bookmarks', 'Edit bookmarks', 'Add, rename or remove the table of contents.', 'outline toc chapters', 'server'],
  ]),
  ...add('pdf-security', [
    ['protect-pdf', 'Protect with password', 'Lock a PDF with AES-256 and choose what readers may do.', 'encrypt lock password secure', 'server'],
    ['unlock-pdf', 'Unlock PDF', 'Remove the password from a PDF you can open.', 'decrypt remove password', 'server'],
    ['sign-pdf', 'Sign', 'Draw, type or upload a signature and place it.', 'signature e-sign', 'server'],
    ['cert-sign', 'Sign with certificate', 'Apply a digital signature from a certificate file.', 'digital signature p12 pfx', 'server'],
    ['validate-signature', 'Check signatures', 'Verify the digital signatures in a PDF.', 'verify signature', 'server'],
    ['redact-pdf', 'Redact', 'Black out text or areas permanently.', 'censor black out hide', 'server'],
    ['sanitize-pdf', 'Sanitise', 'Remove metadata, scripts, links and hidden data.', 'clean privacy scrub', 'server'],
    ['show-javascript', 'Show JavaScript', 'List any scripts embedded in a PDF.', 'scripts security', 'server'],
  ]),
  ...add('pdf-optimise', [
    ['compress-pdf', 'Compress PDF', 'Make a PDF smaller; optionally re-compress photos.', 'shrink reduce size optimise optimize smaller', 'server'],
    ['repair-pdf', 'Repair PDF', 'Rebuild a damaged PDF that won’t open properly.', 'fix broken corrupt recover', 'server'],
    ['pdf-info', 'PDF info', 'Pages, sizes, version, encryption and metadata at a glance.', 'properties inspect details', 'server'],
    ['pdf-a', 'Convert to PDF/A', 'Make an archive-ready PDF/A file.', 'archive pdfa', 'office'],
  ]),
  ...add('pdf-convert', [
    ['images-to-pdf', 'Images to PDF', 'Turn JPG, PNG, WebP and other images into a PDF, one per page.', 'jpg png webp photo scan convert', 'server'],
    ['pdf-to-images', 'PDF to images', 'Save each page as a PNG or JPG.', 'png jpg export render convert'],
    ['pdf-to-text', 'PDF to text', 'Copy out all the text in a PDF.', 'extract text txt copy'],
    ['pdf-to-markdown', 'PDF to Markdown', 'Convert a PDF’s text into Markdown.', 'md convert'],
    ['extract-images', 'Extract images', 'Save every picture embedded in a PDF.', 'pictures photos export', 'server'],
    ['office-to-pdf', 'Office to PDF', 'Convert Word, Excel, PowerPoint and OpenDocument files.', 'word docx excel xlsx powerpoint pptx odt', 'office'],
    ['pdf-to-office', 'PDF to Word or PowerPoint', 'Turn a PDF into an editable Word or PowerPoint file.', 'word docx powerpoint pptx odt edit', 'office'],
    ['html-to-pdf', 'Web page to PDF', 'Save a URL or HTML file as a PDF.', 'url html website', 'office'],
    ['markdown-to-pdf', 'Markdown to PDF', 'Render Markdown as a tidy PDF.', 'md', 'office'],
    ['epub-to-pdf', 'EPUB to PDF', 'Turn an e-book into a PDF you can print or read anywhere.', 'epub ebook book', 'office'],
    ['pdf-to-epub', 'PDF to EPUB', 'Make a reflowable e-book from a PDF’s text.', 'epub ebook kindle reader'],
    ['ocr-pdf', 'Make searchable (OCR)', 'Recognise text in scanned PDFs so you can search and copy it.', 'ocr scan recognise text tesseract', 'ocr'],
    ['pdf-pipeline', 'Pipelines', 'Chain several PDF tools and save the recipe.', 'automate workflow batch', 'server'],
  ]),
  ...add('dev-crypto', [
    ['token-generator', 'Token and password generator', 'Random strings with the characters and length you choose.', 'password random secret string key'],
    ['hash-text', 'Hash text', 'MD5, SHA-1, SHA-2, SHA-3 and BLAKE3 of any text.', 'md5 sha sha256 sha512 sha3 blake3 digest checksum'],
    ['bcrypt', 'Bcrypt', 'Hash a password with bcrypt, or check one against a hash.', 'password hash compare verify'],
    ['uuid-generator', 'UUID generator', 'Version 4 (random) and version 7 (time-ordered) UUIDs.', 'guid uuid4 uuid7 id'],
    ['ulid-generator', 'ULID generator', 'Sortable, URL-safe unique IDs.', 'id unique sortable'],
    ['hmac-generator', 'HMAC generator', 'Sign text with a secret key using SHA-1/256/384/512.', 'signature mac sha256 webhook'],
    ['encrypt-text', 'Encrypt and decrypt text', 'AES-GCM with a passphrase.', 'aes cipher secret'],
    ['rsa-keys', 'RSA key pair', 'Generate a public/private key pair in PEM.', 'rsa pem ssh keys'],
    ['bip39', 'Passphrase generator', 'BIP39 word lists for memorable passphrases.', 'mnemonic words seed'],
    ['password-strength', 'Password strength', 'How long a password would take to crack.', 'entropy crack'],
    ['otp', 'One-time codes (TOTP)', 'Generate and check authenticator codes.', 'totp 2fa mfa authenticator'],
  ]),
  ...add('dev-convert', [
    ['json-yaml', 'JSON and YAML', 'Convert JSON to YAML and back.', 'yml convert'],
    ['json-toml', 'JSON and TOML', 'Convert JSON to TOML and back.', 'toml convert config'],
    ['json-xml', 'JSON and XML', 'Convert JSON to XML and back.', 'xml convert'],
    ['yaml-toml', 'YAML and TOML', 'Convert YAML to TOML and back.', 'yml toml convert'],
    ['json-csv', 'JSON to CSV', 'Turn an array of objects into a spreadsheet-ready CSV.', 'csv excel table spreadsheet'],
    ['base64-text', 'Base64 text', 'Encode or decode Base64 (standard or URL-safe).', 'encode decode b64 base64url'],
    ['base64-file', 'Base64 file', 'Turn a file into Base64 or a data URL, and back.', 'data url encode decode file'],
    ['date-time', 'Date and time converter', 'Unix timestamps, ISO 8601, RFC 2822 and your local time.', 'timestamp epoch unix iso date time timezone'],
    ['number-base', 'Number base converter', 'Binary, octal, decimal, hex or any base from 2 to 36.', 'binary hex hexadecimal octal decimal radix'],
    ['roman-numerals', 'Roman numerals', 'Convert numbers to Roman numerals and back.', 'roman'],
    ['color-converter', 'Colour converter', 'Hex, RGB, HSL and OKLCH, with a live swatch.', 'color hex rgb hsl oklch picker'],
    ['case-converter', 'Case converter', 'camelCase, snake_case, kebab-case, Title Case and more.', 'camel snake kebab pascal upper lower title'],
    ['text-binary', 'Text to binary, Unicode and NATO', 'Spell text as bits, code points or the phonetic alphabet.', 'binary unicode nato phonetic ascii'],
    ['list-converter', 'List tools', 'Sort, dedupe, trim, wrap and join lists.', 'sort unique lines join'],
    ['markdown-html', 'Markdown to HTML', 'Render Markdown and copy the HTML.', 'md html'],
  ]),
  ...add('dev-web', [
    ['url-encode', 'URL encode and decode', 'Percent-encode text for URLs, or decode it.', 'percent encoding uri component'],
    ['url-parser', 'URL parser', 'Break a URL into protocol, host, path and query parameters.', 'query string params parse'],
    ['html-entities', 'HTML entities', 'Escape or unescape <, &, quotes and friends.', 'escape unescape html encode'],
    ['jwt-parser', 'JWT decoder', 'Read the header and claims of a JSON Web Token.', 'jwt token bearer decode claims'],
    ['http-status', 'HTTP status codes', 'What every status code means.', 'http 404 500 status code reference'],
    ['slugify', 'Slugify', 'Make URL-friendly slugs from any text.', 'slug url permalink'],
    ['basic-auth', 'Basic auth header', 'Build an Authorization header from a username and password.', 'authorization header'],
    ['user-agent', 'User-agent parser', 'Identify the browser, OS and device from a user-agent string.', 'browser ua'],
    ['mime-types', 'MIME types', 'Look up a file extension’s content type.', 'content type extension'],
    ['meta-tags', 'Meta tag generator', 'Open Graph and Twitter card tags for a page.', 'og opengraph seo'],
    ['keycode', 'Keycode info', 'Press a key to see its key, code and keyCode.', 'keyboard event'],
    ['safelink', 'Safelink decoder', 'Recover the real URL from an Outlook safelink.', 'outlook microsoft link'],
    ['device-info', 'Device information', 'Screen, browser and device details.', 'screen browser'],
    ['html-editor', 'HTML editor', 'Write rich text and copy the HTML.', 'wysiwyg rich text'],
  ]),
  ...add('dev-development', [
    ['json-formatter', 'JSON formatter', 'Format, validate and minify JSON.', 'prettify pretty beautify minify validate lint'],
    ['json-diff', 'JSON diff', 'Compare two JSON documents.', 'compare difference'],
    ['sql-formatter', 'SQL formatter', 'Tidy up SQL queries.', 'prettify beautify query'],
    ['xml-formatter', 'XML formatter', 'Indent and validate XML.', 'prettify beautify'],
    ['yaml-viewer', 'YAML formatter', 'Validate and tidy YAML.', 'yml prettify'],
    ['regex-tester', 'Regex tester', 'Try a regular expression and see every match and group.', 'regexp regular expression match test'],
    ['regex-cheatsheet', 'Regex cheat sheet', 'The regex syntax you keep forgetting.', 'regexp reference'],
    ['crontab', 'Cron explainer', 'Read a cron expression in plain English and see the next runs.', 'cron schedule crontab job'],
    ['chmod-calculator', 'chmod calculator', 'Unix permissions as checkboxes, octal and rwx.', 'permissions unix linux rwx octal'],
    ['docker-compose', 'docker run to Compose', 'Turn a docker run command into a compose file.', 'docker compose convert'],
    ['git-cheatsheet', 'Git cheat sheet', 'Common git commands.', 'git reference'],
    ['random-port', 'Random port', 'Pick an unused-looking port number.', 'port network'],
    ['email-normaliser', 'Email normaliser', 'Normalise addresses to spot duplicates.', 'email gmail dots plus'],
    ['benchmark', 'Benchmark builder', 'Compare timings side by side.', 'performance timing'],
  ]),
  ...add('dev-network', [
    ['ipv4-subnet', 'IPv4 subnet calculator', 'Network, broadcast, host range and mask for any CIDR.', 'cidr subnet netmask network ip range'],
    ['ipv4-convert', 'IPv4 address converter', 'An IP as decimal, hex, binary and IPv6.', 'ip convert'],
    ['ipv4-range', 'IPv4 range expander', 'Smallest CIDR covering a range of IPs.', 'cidr range'],
    ['ipv6-ula', 'IPv6 ULA generator', 'Random unique local IPv6 prefixes.', 'ipv6 local'],
    ['mac-generator', 'MAC address generator', 'Random MAC addresses.', 'mac random'],
    ['mac-lookup', 'MAC vendor lookup', 'Find the manufacturer of a MAC address.', 'oui vendor', 'server'],
  ]),
  ...add('text', [
    ['text-diff', 'Text diff', 'Compare two texts and see what changed, line by line.', 'compare difference changes'],
    ['text-stats', 'Text statistics', 'Characters, words, lines, sentences and reading time.', 'count words characters length'],
    ['lorem-ipsum', 'Lorem ipsum', 'Placeholder paragraphs, sentences or words.', 'placeholder dummy filler text'],
    ['qr-code', 'QR code generator', 'QR codes for text or links, as PNG or SVG.', 'qr barcode link'],
    ['wifi-qr', 'Wi-Fi QR code', 'Let guests join your Wi-Fi by scanning a code.', 'wifi wireless guest network qr'],
    ['string-obfuscator', 'String obfuscator', 'Hide most of a secret, keep the ends.', 'mask hide'],
    ['numeronym', 'Numeronym', 'Turn "internationalisation" into "i18n".', 'abbreviation'],
    ['ascii-art', 'ASCII art', 'Big letters made of characters.', 'figlet banner'],
    ['emoji', 'Emoji picker', 'Search and copy emoji.', 'emoji smiley'],
    ['math-eval', 'Maths evaluator', 'Type an expression, get the answer.', 'calculator math'],
    ['percentage', 'Percentage calculator', 'X% of Y, and percentage change.', 'percent'],
    ['eta', 'ETA calculator', 'When will this job finish?', 'time estimate'],
    ['temperature', 'Unit converter', 'Temperature, length, weight and more.', 'celsius fahrenheit units convert'],
    ['chronometer', 'Stopwatch', 'A simple stopwatch.', 'timer stopwatch'],
    ['phone-parser', 'Phone number parser', 'Validate and format phone numbers.', 'phone telephone'],
    ['iban', 'IBAN validator', 'Check and break down an IBAN.', 'bank account'],
    ['svg-placeholder', 'SVG placeholder', 'Placeholder images of any size.', 'image placeholder'],
  ]),
  ...add('files', [
    ['data-convert', 'CSV, JSON, XML and YAML', 'Convert data files between formats.', 'csv json xml yaml convert'],
    ['csv-merge-split', 'Merge or split CSV', 'Join CSV files or split a big one.', 'csv'],
    ['excel-csv', 'Excel and CSV', 'Convert spreadsheets to CSV and back.', 'xlsx excel spreadsheet'],
    ['office-convert', 'Office format converter', 'Switch documents, spreadsheets and slides between Word, OpenDocument and other formats.', 'docx odt doc xlsx ods xls csv pptx odp ppt convert word excel powerpoint', 'office'],
    ['chart-maker', 'Chart maker', 'Make a chart from a CSV.', 'graph plot csv'],
    ['zip-tool', 'Zip and unzip', 'Create or extract ZIP archives.', 'archive compress unzip'],
    ['file-checksum', 'File checksum', 'SHA-256 and other hashes of a file.', 'hash sha256 md5 verify download'],
    ['file-type', 'Identify a file', 'Work out what a file really is.', 'magic mime'],
  ]),
  ...add('image', [
    ['image-ocr', 'Text from images (OCR)', 'Copy text out of a photo, screenshot or scan, in any of 160 languages.', 'ocr recognise read scan screenshot tesseract', 'ocr'],
    ['image-resize', 'Resize images', 'Change pixel size, in bulk.', 'scale dimensions', 'server'],
    ['image-crop', 'Crop images', 'Cut images to a size or ratio.', 'trim aspect', 'server'],
    ['image-rotate', 'Rotate and flip', 'Turn or mirror images.', 'flip mirror', 'server'],
    ['image-compress', 'Compress images', 'Smaller JPG, PNG, WebP and AVIF files.', 'optimise shrink', 'server'],
    ['image-convert', 'Convert images', 'Between JPG, PNG, WebP, AVIF, GIF, TIFF, BMP and ICO. Opens HEIC too.', 'heic webp avif png jpg', 'server'],
    ['exif', 'View and strip EXIF', 'See photo metadata and remove location data.', 'metadata gps location privacy', 'server'],
    ['image-watermark', 'Watermark images', 'Add text or a logo to photos.', 'logo stamp', 'server'],
    ['image-border', 'Borders and padding', 'Add a frame or pad to a size.', 'frame', 'server'],
    ['image-adjust', 'Adjust colours', 'Brightness, contrast and saturation.', 'brightness contrast', 'server'],
    ['image-filters', 'Grayscale and filters', 'Black and white, sepia, invert.', 'grayscale sepia', 'server'],
    ['image-blur', 'Blur or pixelate', 'Hide faces, plates or text.', 'redact pixelate', 'server'],
    ['collage', 'Collage', 'Arrange photos in a grid.', 'grid', 'server'],
    ['sprite-sheet', 'Sprite sheet', 'Pack images into one sheet.', 'atlas', 'server'],
    ['favicon', 'Favicon generator', 'Every icon size a website needs.', 'icon ico', 'server'],
    ['palette', 'Colour palette', 'Pull the main colours from an image.', 'colors extract'],
    ['image-base64', 'Image to Base64', 'Data URLs for images.', 'data url'],
    ['svg-optimise', 'Optimise SVG', 'Make SVG files smaller.', 'svgo'],
    ['svg-to-png', 'SVG to PNG', 'Render SVG at any size.', 'rasterise'],
    ['gif-maker', 'GIF maker', 'Make an animated GIF from images.', 'animation', 'server'],
    ['screenshot-beautifier', 'Screenshot beautifier', 'Frame screenshots with a background and shadow.', 'mockup'],
    ['meme', 'Meme maker', 'Top and bottom captions.', 'caption'],
    ['passport-photo', 'Passport photo', 'Crop to passport sizes and tile for printing.', 'id photo', 'server'],
    ['duplicates', 'Find duplicate images', 'Spot near-identical photos.', 'dedupe similar'],
    ['vectorise', 'Vectorise', 'Trace a bitmap into SVG.', 'trace svg', 'server'],
    ['image-editor', 'Image editor', 'Layers, brushes, shapes and filters.', 'paint draw layers'],
  ]),
  ...add('media', [
    ['video-convert', 'Convert video', 'MOV, MKV, WebM, AVI to MP4 and more.', 'mp4 mov mkv webm', 'media'],
    ['video-compress', 'Compress video', 'Smaller video files.', 'shrink', 'media'],
    ['video-trim', 'Trim video', 'Cut the start and end.', 'cut clip', 'media'],
    ['video-merge', 'Merge videos', 'Join clips together.', 'join concat', 'media'],
    ['video-resize', 'Resize video', 'Change resolution.', '1080p 720p scale', 'media'],
    ['video-crop-rotate', 'Crop and rotate video', 'Fix sideways phone videos.', 'rotate crop', 'media'],
    ['video-speed', 'Change speed', 'Speed up, slow down or change FPS.', 'fps slow motion timelapse', 'media'],
    ['video-mute', 'Mute video', 'Remove the audio track.', 'silent', 'media'],
    ['extract-audio', 'Extract audio', 'Save the soundtrack as MP3 or M4A.', 'mp3 audio', 'media'],
    ['video-gif', 'Video to GIF', 'Turn a clip into a GIF.', 'gif animation', 'media'],
    ['video-stabilise', 'Stabilise video', 'Smooth out shaky footage.', 'shake', 'media'],
    ['subtitles', 'Subtitles', 'Burn in or extract subtitles.', 'srt captions', 'media'],
    ['video-thumbnails', 'Thumbnails', 'Grab frames or a contact sheet.', 'frames screenshot', 'media'],
    ['audio-convert', 'Convert audio', 'M4A, AAC, OGG, FLAC, WAV and MP3.', 'mp3 wav flac', 'media'],
    ['audio-trim', 'Trim audio', 'Cut the start and end.', 'cut', 'media'],
    ['audio-merge-split', 'Merge or split audio', 'Join tracks or split one.', 'join', 'media'],
    ['audio-normalise', 'Normalise loudness', 'Even out volume levels.', 'loudness', 'media'],
    ['audio-volume-fade', 'Volume and fades', 'Louder, quieter, fade in and out.', 'fade', 'media'],
    ['audio-pitch', 'Pitch shift', 'Change pitch without changing speed.', 'pitch', 'media'],
    ['silence-removal', 'Remove silence', 'Cut the quiet gaps.', 'silence', 'media'],
    ['noise-reduction', 'Reduce noise', 'Clean up background hiss.', 'denoise', 'media'],
    ['waveform', 'Waveform image', 'Draw a track’s waveform.', 'visualise', 'media'],
  ]),
  ...add('ai', [
    ['remove-background', 'Remove background', 'Cut the subject out of a photo.', 'background cutout transparent', 'ai'],
    ['upscale', 'Upscale image', 'Enlarge photos with added detail.', 'enlarge super resolution', 'ai'],
    ['restore-photo', 'Restore old photos', 'Repair and colourise old photos.', 'colorize restore', 'ai'],
    ['erase-object', 'Erase objects', 'Remove unwanted things from a photo.', 'inpaint remove', 'ai'],
    ['faces', 'Blur or enhance faces', 'Find faces and blur or sharpen them.', 'privacy face', 'ai'],
    ['transcribe', 'Transcribe audio', 'Speech to text with Whisper.', 'whisper speech text', 'ai'],
    ['auto-subtitles', 'Auto subtitles', 'Generate subtitles for a video.', 'captions srt', 'ai'],
    ['expand-canvas', 'Expand canvas', 'Extend a photo beyond its edges.', 'outpaint', 'ai'],
  ]),
];

export const toolById = new Map(tools.map((t) => [t.id, t]));
export const categoryById = new Map(categories.map((c) => [c.id, c]));

export const runtimeNote: Record<Runtime, string> = {
  browser: 'Runs in your browser. Nothing is uploaded.',
  server: 'Processed on your Ferrokit server. Files are deleted as soon as the result is sent.',
  addon: 'Needs an add-on container on your Ferrokit server.',
};
