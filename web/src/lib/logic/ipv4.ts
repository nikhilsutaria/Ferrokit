// IPv4 subnet maths on unsigned 32-bit numbers.

export function parseIp(text: string): number {
  const s = text.trim();
  const parts = s.split('.');
  if (parts.length !== 4) throw new Error(`“${s}” isn’t an IPv4 address; it needs four numbers separated by dots, like 192.168.1.10.`);
  let n = 0;
  for (const p of parts) {
    if (!/^\d{1,3}$/.test(p)) throw new Error(`“${p}” in “${s}” isn’t a number from 0 to 255.`);
    if (p.length > 1 && p.startsWith('0')) throw new Error(`“${p}” has a leading zero, which some systems read as octal; write it without the zero.`);
    const v = Number(p);
    if (v > 255) throw new Error(`${v} is too big; each part of an address goes up to 255.`);
    n = n * 256 + v;
  }
  return n;
}

export function formatIp(n: number): string {
  return [n >>> 24, (n >>> 16) & 255, (n >>> 8) & 255, n & 255].join('.');
}

export function prefixToMask(prefix: number): number {
  return prefix === 0 ? 0 : (0xffffffff << (32 - prefix)) >>> 0;
}

export function maskToPrefix(mask: number): number {
  const inv = ~mask >>> 0;
  // A valid mask's inverse is 2^k - 1.
  if ((inv & (inv + 1)) !== 0) throw new Error(`${formatIp(mask)} isn’t a valid netmask; its 1 bits must all come first, as in 255.255.255.0.`);
  let prefix = 0;
  for (let m = mask; m & 0x80000000; m = (m << 1) >>> 0) prefix++;
  return prefix;
}

export function toBinary(n: number): string {
  return (n >>> 0).toString(2).padStart(32, '0').replace(/(.{8})(?!$)/g, '$1.');
}

/** Parse "10.0.0.1/8", "10.0.0.1/255.0.0.0", "10.0.0.1 255.0.0.0" or a bare address (treated as /32). */
export function parseCidr(text: string): { ip: number; prefix: number } {
  const s = text.trim();
  if (!s) throw new Error('Enter an address such as 192.168.1.10/24.');
  const m = /^([^\s/]+)\s*(?:[/\s]\s*(\S+))?$/.exec(s);
  if (!m) throw new Error('Write it as an address and prefix, such as 192.168.1.10/24, or an address and mask.');
  const ip = parseIp(m[1]);
  const rest = m[2];
  if (rest === undefined) return { ip, prefix: 32 };
  if (/^\d{1,2}$/.test(rest)) {
    const prefix = Number(rest);
    if (prefix > 32) throw new Error(`The prefix length must be from 0 to 32; got /${prefix}.`);
    return { ip, prefix };
  }
  if (rest.includes('.')) return { ip, prefix: maskToPrefix(parseIp(rest)) };
  throw new Error(`“${rest}” isn’t a prefix length (0–32) or a netmask like 255.255.255.0.`);
}

export interface SubnetInfo {
  address: string;
  prefix: number;
  cidr: string;
  network: string;
  /** null for /31 and /32, which have no broadcast address. */
  broadcast: string | null;
  firstHost: string;
  lastHost: string;
  hosts: number;
  totalAddresses: number;
  netmask: string;
  wildcard: string;
  binaryNetmask: string;
  binaryAddress: string;
  ipClass: string;
  scope: string;
}

export function ipClass(ip: number): string {
  const a = ip >>> 24;
  if (a < 128) return 'A';
  if (a < 192) return 'B';
  if (a < 224) return 'C';
  if (a < 240) return 'D (multicast)';
  return 'E (reserved)';
}

const SCOPES: [string, number, string][] = [
  ['0.0.0.0', 8, 'This network (unspecified)'],
  ['10.0.0.0', 8, 'Private (RFC 1918)'],
  ['100.64.0.0', 10, 'Shared address space (carrier-grade NAT)'],
  ['127.0.0.0', 8, 'Loopback'],
  ['169.254.0.0', 16, 'Link-local'],
  ['172.16.0.0', 12, 'Private (RFC 1918)'],
  ['192.0.0.0', 24, 'IETF protocol assignments'],
  ['192.0.2.0', 24, 'Documentation (TEST-NET-1)'],
  ['192.168.0.0', 16, 'Private (RFC 1918)'],
  ['198.18.0.0', 15, 'Benchmark testing'],
  ['198.51.100.0', 24, 'Documentation (TEST-NET-2)'],
  ['203.0.113.0', 24, 'Documentation (TEST-NET-3)'],
  ['224.0.0.0', 4, 'Multicast'],
  ['255.255.255.255', 32, 'Limited broadcast'],
  ['240.0.0.0', 4, 'Reserved'],
];

export function scope(ip: number): string {
  for (const [net, prefix, name] of SCOPES) {
    const mask = prefixToMask(prefix);
    if (((ip & mask) >>> 0) === parseIp(net)) return name;
  }
  return 'Public';
}

export function subnet(ip: number, prefix: number): SubnetInfo {
  const mask = prefixToMask(prefix);
  const network = (ip & mask) >>> 0;
  const broadcast = (network | ~mask) >>> 0;
  const total = 2 ** (32 - prefix);
  let first = network;
  let last = broadcast;
  let hosts = total;
  if (prefix <= 30) {
    first = network + 1;
    last = broadcast - 1;
    hosts = total - 2;
  }
  return {
    address: formatIp(ip),
    prefix,
    cidr: `${formatIp(network)}/${prefix}`,
    network: formatIp(network),
    broadcast: prefix <= 30 ? formatIp(broadcast) : null,
    firstHost: formatIp(first),
    lastHost: formatIp(last),
    hosts,
    totalAddresses: total,
    netmask: formatIp(mask),
    wildcard: formatIp(~mask >>> 0),
    binaryNetmask: toBinary(mask),
    binaryAddress: toBinary(ip),
    ipClass: ipClass(ip),
    scope: scope(ip),
  };
}
