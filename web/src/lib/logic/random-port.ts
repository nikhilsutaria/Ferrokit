// Random TCP/UDP port numbers that steer clear of ports popular software already uses.

export const RANGES = {
  registered: [1024, 49151],
  dynamic: [49152, 65535],
} as const satisfies Record<string, readonly [number, number]>;

export const MAX_COUNT = 20;

/** Ports that common servers, databases and dev tools listen on by default. */
export const AVOIDED_PORTS: [number, string][] = [
  [20, 'FTP data'],
  [21, 'FTP'],
  [22, 'SSH'],
  [23, 'Telnet'],
  [25, 'SMTP'],
  [53, 'DNS'],
  [67, 'DHCP'],
  [69, 'TFTP'],
  [80, 'HTTP'],
  [110, 'POP3'],
  [123, 'NTP'],
  [143, 'IMAP'],
  [161, 'SNMP'],
  [389, 'LDAP'],
  [443, 'HTTPS'],
  [445, 'SMB'],
  [465, 'SMTP over TLS'],
  [514, 'Syslog'],
  [587, 'SMTP submission'],
  [636, 'LDAP over TLS'],
  [853, 'DNS over TLS'],
  [993, 'IMAP over TLS'],
  [995, 'POP3 over TLS'],
  [1080, 'SOCKS proxy'],
  [1194, 'OpenVPN'],
  [1433, 'SQL Server'],
  [1434, 'SQL Server Browser'],
  [1521, 'Oracle Database'],
  [1701, 'L2TP'],
  [1723, 'PPTP'],
  [1812, 'RADIUS'],
  [1813, 'RADIUS accounting'],
  [1883, 'MQTT'],
  [1900, 'UPnP / SSDP'],
  [2049, 'NFS'],
  [2181, 'ZooKeeper'],
  [2283, 'Immich'],
  [2375, 'Docker API'],
  [2376, 'Docker API over TLS'],
  [2379, 'etcd'],
  [2380, 'etcd peers'],
  [3000, 'Node.js, Rails and Grafana dev servers'],
  [3001, 'Dev servers'],
  [3100, 'Grafana Loki'],
  [3128, 'Squid proxy'],
  [3268, 'Active Directory catalogue'],
  [3306, 'MySQL and MariaDB'],
  [3389, 'Remote Desktop'],
  [3478, 'STUN / TURN'],
  [4000, 'Jekyll and Phoenix dev servers'],
  [4040, 'Spark UI'],
  [4200, 'Angular dev server'],
  [4317, 'OpenTelemetry gRPC'],
  [4318, 'OpenTelemetry HTTP'],
  [4369, 'Erlang port mapper'],
  [4443, 'HTTPS alternative'],
  [5000, 'Flask, ASP.NET and macOS AirPlay'],
  [5001, 'ASP.NET HTTPS'],
  [5005, 'Java debugger'],
  [5060, 'SIP'],
  [5061, 'SIP over TLS'],
  [5173, 'Vite dev server'],
  [5353, 'mDNS'],
  [5432, 'PostgreSQL'],
  [5433, 'PostgreSQL (second instance)'],
  [5601, 'Kibana'],
  [5672, 'RabbitMQ'],
  [5900, 'VNC'],
  [5938, 'TeamViewer'],
  [5984, 'CouchDB'],
  [6000, 'X11'],
  [6006, 'TensorBoard'],
  [6379, 'Redis'],
  [6443, 'Kubernetes API'],
  [6667, 'IRC'],
  [6881, 'BitTorrent'],
  [7000, 'Cassandra and macOS AirPlay'],
  [7001, 'Cassandra over TLS'],
  [7077, 'Spark'],
  [7474, 'Neo4j'],
  [7687, 'Neo4j Bolt'],
  [7878, 'Radarr'],
  [8000, 'Django and Python http.server'],
  [8001, 'kubectl proxy'],
  [8008, 'HTTP alternative'],
  [8080, 'HTTP alternative, Tomcat and proxies'],
  [8081, 'HTTP alternative'],
  [8086, 'InfluxDB'],
  [8088, 'HTTP alternative'],
  [8096, 'Jellyfin'],
  [8123, 'Home Assistant'],
  [8200, 'Vault'],
  [8384, 'Syncthing'],
  [8443, 'HTTPS alternative'],
  [8500, 'Consul'],
  [8883, 'MQTT over TLS'],
  [8888, 'Jupyter'],
  [8983, 'Solr'],
  [8989, 'Sonarr'],
  [9000, 'PHP-FPM, MinIO and SonarQube'],
  [9001, 'MinIO console'],
  [9042, 'Cassandra CQL'],
  [9090, 'Prometheus and Cockpit'],
  [9091, 'Transmission'],
  [9092, 'Kafka'],
  [9093, 'Alertmanager'],
  [9100, 'Node exporter and printers'],
  [9200, 'Elasticsearch'],
  [9229, 'Node.js inspector'],
  [9300, 'Elasticsearch nodes'],
  [9411, 'Zipkin'],
  [9418, 'git daemon'],
  [9443, 'Portainer'],
  [10000, 'Webmin'],
  [11211, 'Memcached'],
  [11434, 'Ollama'],
  [15672, 'RabbitMQ management'],
  [16686, 'Jaeger'],
  [19999, 'Netdata'],
  [22000, 'Syncthing sync'],
  [25565, 'Minecraft'],
  [27017, 'MongoDB'],
  [27018, 'MongoDB shard'],
  [27019, 'MongoDB config server'],
  [32400, 'Plex'],
  [51820, 'WireGuard'],
  [61616, 'ActiveMQ'],
];

const AVOIDED = new Set(AVOIDED_PORTS.map(([p]) => p));

export function isAvoided(port: number): boolean {
  return AVOIDED.has(port);
}

/** Avoided ports inside a range, for showing what was left out. */
export function avoidedIn(min: number, max: number): [number, string][] {
  return AVOIDED_PORTS.filter(([p]) => p >= min && p <= max);
}

export function usableCount(min: number, max: number): number {
  return max - min + 1 - avoidedIn(min, max).length;
}

/** Uniform random integer in [0, n), without modulo bias. */
function cryptoInt(n: number): number {
  const limit = 2 ** 32 - (2 ** 32 % n);
  const buf = new Uint32Array(1);
  for (;;) {
    crypto.getRandomValues(buf);
    if (buf[0] < limit) return buf[0] % n;
  }
}

/** `count` distinct random ports in [min, max], none of them in the avoided list. */
export function pickPorts(min: number, max: number, count: number, randomInt: (n: number) => number = cryptoInt): number[] {
  if (!Number.isInteger(min) || !Number.isInteger(max) || min < 1 || max > 65535) {
    throw new Error('Ports are whole numbers between 1 and 65535.');
  }
  if (min > max) throw new Error('The lowest port must be no higher than the highest.');
  if (!Number.isInteger(count) || count < 1 || count > MAX_COUNT) throw new Error(`Ask for 1 to ${MAX_COUNT} ports.`);

  const pool: number[] = [];
  for (let p = min; p <= max; p++) if (!AVOIDED.has(p)) pool.push(p);
  if (pool.length < count) {
    throw new Error(
      pool.length === 0
        ? 'There are no usable ports in this range. Widen the range.'
        : `There are only ${pool.length} usable ports in this range. Widen the range or ask for fewer.`,
    );
  }
  // Partial Fisher–Yates: the first `count` slots end up as a uniform random sample.
  for (let i = 0; i < count; i++) {
    const j = i + randomInt(pool.length - i);
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  return pool.slice(0, count);
}

export function checkCommands(port: number): { system: string; command: string }[] {
  return [
    { system: 'Linux', command: `ss -ltn 'sport = :${port}'` },
    { system: 'macOS', command: `lsof -i :${port}` },
    { system: 'Windows', command: `netstat -ano | findstr :${port}` },
  ];
}
