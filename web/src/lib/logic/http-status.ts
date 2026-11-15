// Standard HTTP status codes (IANA registry, RFC 9110 and friends) with plain explanations.

export interface HttpStatus {
  code: number;
  name: string;
  description: string;
}

export interface StatusClass {
  digit: number;
  name: string;
  summary: string;
}

export const statusClasses: StatusClass[] = [
  { digit: 1, name: 'Informational', summary: 'The request was received and is still being processed.' },
  { digit: 2, name: 'Success', summary: 'The request worked.' },
  { digit: 3, name: 'Redirection', summary: 'The client needs to go somewhere else to finish the request.' },
  { digit: 4, name: 'Client error', summary: 'Something is wrong with the request itself.' },
  { digit: 5, name: 'Server error', summary: 'The request looked fine, but the server failed to handle it.' },
];

export const statuses: HttpStatus[] = [
  { code: 100, name: 'Continue', description: 'The server got the request headers and the client should go ahead and send the body.' },
  { code: 101, name: 'Switching Protocols', description: 'The server agrees to switch to the protocol the client asked for, such as WebSocket.' },
  { code: 102, name: 'Processing', description: 'The server has accepted a long WebDAV request and is still working on it.' },
  { code: 103, name: 'Early Hints', description: 'The server sends some headers early so the browser can start preloading resources.' },

  { code: 200, name: 'OK', description: 'The request succeeded and the response holds the result.' },
  { code: 201, name: 'Created', description: 'The request succeeded and a new resource was created, usually at the Location header.' },
  { code: 202, name: 'Accepted', description: 'The request was accepted for processing, but the work hasn’t finished yet.' },
  { code: 203, name: 'Non-Authoritative Information', description: 'The response succeeded but a proxy has changed it from what the origin server sent.' },
  { code: 204, name: 'No Content', description: 'The request succeeded and there is nothing to send back.' },
  { code: 205, name: 'Reset Content', description: 'The request succeeded and the client should reset the form or view that sent it.' },
  { code: 206, name: 'Partial Content', description: 'The server is sending only the byte range the client asked for.' },
  { code: 207, name: 'Multi-Status', description: 'A WebDAV response holding separate status codes for several resources.' },
  { code: 208, name: 'Already Reported', description: 'A WebDAV member was already listed earlier in this response, so it isn’t repeated.' },
  { code: 226, name: 'IM Used', description: 'The server applied delta encoding and is returning the changes rather than the full resource.' },

  { code: 300, name: 'Multiple Choices', description: 'There are several versions of the resource and the client should pick one.' },
  { code: 301, name: 'Moved Permanently', description: 'The resource has a new permanent URL; update links and bookmarks.' },
  { code: 302, name: 'Found', description: 'The resource is temporarily at another URL; keep using the original one.' },
  { code: 303, name: 'See Other', description: 'Fetch the result from another URL with GET, typically after a form POST.' },
  { code: 304, name: 'Not Modified', description: 'The cached copy is still current, so the server sends no body.' },
  { code: 305, name: 'Use Proxy', description: 'Deprecated: the resource had to be fetched through a given proxy.' },
  { code: 307, name: 'Temporary Redirect', description: 'Repeat the request at another URL for now, keeping the same method and body.' },
  { code: 308, name: 'Permanent Redirect', description: 'The resource moved for good; repeat the request there with the same method and body.' },

  { code: 400, name: 'Bad Request', description: 'The server can’t understand the request because it is malformed.' },
  { code: 401, name: 'Unauthorized', description: 'The request needs valid credentials; log in or send a token.' },
  { code: 402, name: 'Payment Required', description: 'Reserved for payment systems and used by some APIs when a plan limit or payment is due.' },
  { code: 403, name: 'Forbidden', description: 'The server knows who you are but you aren’t allowed to do this.' },
  { code: 404, name: 'Not Found', description: 'There is nothing at this URL.' },
  { code: 405, name: 'Method Not Allowed', description: 'The URL exists but doesn’t accept this method, such as POST on a read-only resource.' },
  { code: 406, name: 'Not Acceptable', description: 'The server can’t produce a response in any format the Accept headers allow.' },
  { code: 407, name: 'Proxy Authentication Required', description: 'A proxy between you and the server needs credentials first.' },
  { code: 408, name: 'Request Timeout', description: 'The client took too long to send the request, so the server gave up.' },
  { code: 409, name: 'Conflict', description: 'The request clashes with the current state of the resource, such as an edit conflict.' },
  { code: 410, name: 'Gone', description: 'The resource used to exist here and has been removed for good.' },
  { code: 411, name: 'Length Required', description: 'The server needs a Content-Length header on this request.' },
  { code: 412, name: 'Precondition Failed', description: 'A condition in the request headers, such as If-Match, wasn’t met.' },
  { code: 413, name: 'Content Too Large', description: 'The request body is bigger than the server will accept.' },
  { code: 414, name: 'URI Too Long', description: 'The URL is longer than the server will process.' },
  { code: 415, name: 'Unsupported Media Type', description: 'The server doesn’t accept the body’s Content-Type.' },
  { code: 416, name: 'Range Not Satisfiable', description: 'The requested byte range is outside the size of the resource.' },
  { code: 417, name: 'Expectation Failed', description: 'The server can’t meet the Expect header in the request.' },
  { code: 418, name: "I'm a teapot", description: 'An April Fools’ joke from 1998: the server refuses to brew coffee because it is a teapot.' },
  { code: 421, name: 'Misdirected Request', description: 'The request reached a server that can’t answer for this host name.' },
  { code: 422, name: 'Unprocessable Content', description: 'The request is well formed but its content fails validation.' },
  { code: 423, name: 'Locked', description: 'The WebDAV resource is locked.' },
  { code: 424, name: 'Failed Dependency', description: 'The WebDAV request failed because an earlier request it depends on failed.' },
  { code: 425, name: 'Too Early', description: 'The server won’t risk processing a request that might be replayed from TLS early data.' },
  { code: 426, name: 'Upgrade Required', description: 'The client must switch to a different protocol, named in the Upgrade header.' },
  { code: 428, name: 'Precondition Required', description: 'The server requires a conditional request, such as one with If-Match, to avoid lost updates.' },
  { code: 429, name: 'Too Many Requests', description: 'You’ve sent too many requests in a given time; slow down and retry later.' },
  { code: 431, name: 'Request Header Fields Too Large', description: 'The request headers, or one of them, are too big.' },
  { code: 451, name: 'Unavailable For Legal Reasons', description: 'The resource is blocked for legal reasons, such as a court order or censorship.' },

  { code: 500, name: 'Internal Server Error', description: 'The server hit an unexpected problem and couldn’t finish the request.' },
  { code: 501, name: 'Not Implemented', description: 'The server doesn’t support the feature needed to answer, such as an unknown method.' },
  { code: 502, name: 'Bad Gateway', description: 'A gateway or proxy got an invalid response from the server behind it.' },
  { code: 503, name: 'Service Unavailable', description: 'The server is overloaded or down for maintenance; try again later.' },
  { code: 504, name: 'Gateway Timeout', description: 'A gateway or proxy didn’t get a response from the server behind it in time.' },
  { code: 505, name: 'HTTP Version Not Supported', description: 'The server doesn’t support the HTTP version used in the request.' },
  { code: 506, name: 'Variant Also Negotiates', description: 'The server’s content negotiation is misconfigured and loops on itself.' },
  { code: 507, name: 'Insufficient Storage', description: 'The WebDAV server has run out of space to complete the request.' },
  { code: 508, name: 'Loop Detected', description: 'The WebDAV server found an infinite loop while processing the request.' },
  { code: 510, name: 'Not Extended', description: 'The request needs further extensions before the server can fulfil it.' },
  { code: 511, name: 'Network Authentication Required', description: 'You need to sign in to the network first, as with a hotel or café Wi-Fi portal.' },
];

/** Match on code (prefix, so "4" or "40" works), name or description. */
export function searchStatuses(query: string, list: HttpStatus[] = statuses): HttpStatus[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  if (/^\d+$/.test(q)) return list.filter((s) => String(s.code).startsWith(q));
  const words = q.split(/\s+/);
  return list.filter((s) => {
    const hay = `${s.code} ${s.name} ${s.description}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}
