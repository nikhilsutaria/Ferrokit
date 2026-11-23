// Turn `docker run` commands into a compose.yaml.

import yaml from 'js-yaml';

/* ---------- Tokenizer ---------- */

/** Characters a backslash escapes outside quotes. Anything else keeps its backslash (C:\data). */
const ESCAPABLE = new Set([' ', '\t', '"', "'", '\\', '$', ';', '&', '|', '#', '`']);

/**
 * Split shell text into commands, each a list of words. Understands quotes, line continuations
 * (\ for bash, ^ for cmd, ` for PowerShell), comments, and ; && || | as separators.
 * A `$` the shell would have kept literally (in single quotes, or escaped) becomes `$$`, which is
 * how Compose writes a literal dollar sign.
 */
export function tokenize(text: string): string[][] {
  const src = text.replace(/[\\^`][ \t]*\r?\n/g, ' ');
  const commands: string[][] = [];
  let words: string[] = [];
  let cur: string | null = null;

  const endWord = () => {
    if (cur !== null) words.push(cur);
    cur = null;
  };
  const endCommand = () => {
    endWord();
    if (words.length) commands.push(words);
    words = [];
  };
  const unclosed = () => new Error('A quote isn’t closed. Check that every " and \' has a partner.');

  for (let i = 0; i < src.length; i++) {
    const ch = src[i];
    if (ch === ' ' || ch === '\t') endWord();
    else if (ch === '\n' || ch === '\r' || ch === ';') endCommand();
    else if (ch === '&' || ch === '|') {
      endCommand();
      if (src[i + 1] === ch) i++;
    } else if (ch === '#' && cur === null) {
      while (i + 1 < src.length && src[i + 1] !== '\n') i++;
    } else if (ch === "'") {
      const end = src.indexOf("'", i + 1);
      if (end < 0) throw unclosed();
      cur = (cur ?? '') + src.slice(i + 1, end).replace(/\$/g, '$$$$');
      i = end;
    } else if (ch === '"') {
      let s = '';
      let j = i + 1;
      for (; j < src.length && src[j] !== '"'; j++) {
        if (src[j] === '\\' && j + 1 < src.length && '"\\$`'.includes(src[j + 1])) {
          j++;
          s += src[j] === '$' ? '$$' : src[j];
        } else s += src[j];
      }
      if (j >= src.length) throw unclosed();
      cur = (cur ?? '') + s;
      i = j;
    } else if (ch === '\\' && i + 1 < src.length && ESCAPABLE.has(src[i + 1])) {
      i++;
      cur = (cur ?? '') + (src[i] === '$' ? '$$' : src[i]);
    } else cur = (cur ?? '') + ch;
  }
  endCommand();
  return commands;
}

/** Pick out the arguments of each docker run (or docker container run, podman run) command. */
export function splitCommands(tokenized: string[][]): { commands: string[][]; skipped: string[] } {
  const commands: string[][] = [];
  const skipped: string[] = [];
  for (const t of tokenized) {
    const w = t[0] === 'sudo' ? t.slice(1) : t;
    const exe = (w[0] ?? '').replace(/\.exe$/i, '').toLowerCase();
    if ((exe === 'docker' || exe === 'podman') && w[1] === 'run') commands.push(w.slice(2));
    else if ((exe === 'docker' || exe === 'podman') && w[1] === 'container' && w[2] === 'run') commands.push(w.slice(3));
    else skipped.push(t.join(' '));
  }
  return { commands, skipped };
}

/* ---------- docker run options ---------- */

// Every docker run option, so the image is found even after flags this tool doesn't convert.
const VALUE_FLAGS = new Set(
  (
    'add-host annotation attach blkio-weight blkio-weight-device cap-add cap-drop cgroup-parent cgroupns cidfile ' +
    'cpu-count cpu-percent cpu-period cpu-quota cpu-rt-period cpu-rt-runtime cpu-shares cpus cpuset-cpus cpuset-mems ' +
    'detach-keys device device-cgroup-rule device-read-bps device-read-iops device-write-bps device-write-iops dns ' +
    'dns-option dns-search domainname entrypoint env env-file expose gpus group-add health-cmd health-interval ' +
    'health-retries health-start-interval health-start-period health-timeout hostname io-maxbandwidth io-maxiops ip ' +
    'ip6 ipc isolation kernel-memory label label-file link link-local-ip log-driver log-opt mac-address memory ' +
    'memory-reservation memory-swap memory-swappiness mount name network network-alias oom-score-adj pid pids-limit ' +
    'platform publish pull restart runtime security-opt shm-size stop-signal stop-timeout storage-opt sysctl tmpfs ' +
    'ulimit user userns uts volume volume-driver volumes-from workdir'
  ).split(' '),
);
const BOOL_FLAGS = new Set(
  (
    'detach disable-content-trust help init interactive no-healthcheck oom-kill-disable privileged publish-all ' +
    'quiet read-only rm sig-proxy tty use-api-socket'
  ).split(' '),
);
const SHORT: Record<string, string> = {
  a: 'attach',
  c: 'cpu-shares',
  d: 'detach',
  e: 'env',
  h: 'hostname',
  i: 'interactive',
  l: 'label',
  m: 'memory',
  p: 'publish',
  P: 'publish-all',
  q: 'quiet',
  t: 'tty',
  u: 'user',
  v: 'volume',
  w: 'workdir',
};
const ALIASES: Record<string, string> = { net: 'network', 'net-alias': 'network-alias', 'dns-opt': 'dns-option' };
/** Options that mean nothing in a compose file. */
const DROPPED = new Set(['detach', 'rm']);

interface Option {
  name: string;
  value: string | boolean;
  /** As typed, for "Not converted". */
  raw: string;
}

interface ParsedRun {
  options: Option[];
  image: string | null;
  args: string[];
}

export function parseRunArgs(args: string[]): ParsedRun {
  const options: Option[] = [];
  const needs = (flag: string) => new Error(`${flag} needs a value.`);
  for (let i = 0; i < args.length; i++) {
    const tok = args[i];
    if (tok === '--') return { options, image: args[i + 1] ?? null, args: args.slice(i + 2) };
    if (tok.startsWith('--') && tok.length > 2) {
      const eq = tok.indexOf('=');
      const typed = eq < 0 ? tok.slice(2) : tok.slice(2, eq);
      const inline = eq < 0 ? undefined : tok.slice(eq + 1);
      const name = ALIASES[typed] ?? typed;
      if (VALUE_FLAGS.has(name)) {
        const value = inline ?? args[++i];
        if (value === undefined) throw needs(`--${typed}`);
        options.push({ name, value, raw: inline === undefined ? `${tok} ${value}` : tok });
      } else {
        const value = inline === undefined ? true : BOOL_FLAGS.has(name) ? !/^(false|0)$/i.test(inline) : inline;
        options.push({ name, value, raw: tok });
      }
    } else if (tok.startsWith('-') && tok.length > 1) {
      const chars = tok.slice(1);
      for (let j = 0; j < chars.length; j++) {
        const ch = chars[j];
        const name = SHORT[ch] ?? `-${ch}`;
        if (VALUE_FLAGS.has(name)) {
          const rest = chars.slice(j + 1);
          const value = rest ? rest.replace(/^=/, '') : args[++i];
          if (value === undefined) throw needs(`-${ch}`);
          options.push({ name, value, raw: rest ? `-${ch}${rest}` : `-${ch} ${value}` });
          break;
        }
        options.push({ name, value: true, raw: `-${ch}` });
      }
    } else return { options, image: tok, args: args.slice(i + 1) };
  }
  return { options, image: null, args: [] };
}

/* ---------- Conversion ---------- */

type Svc = Record<string, unknown>;

interface Ctx {
  svc: Svc;
  namedVolumes: Set<string>;
  networks: string[];
  aliases: string[];
  ipv4?: string;
  ipv6?: string;
  /** Raw text of the alias / ip options, in case there's no custom network to put them on. */
  netExtras: string[];
}

const NAME = /^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/;

function push(svc: Svc, key: string, v: unknown) {
  ((svc[key] ??= []) as unknown[]).push(v);
}

function obj(svc: Svc, key: string): Record<string, unknown> {
  return (svc[key] ??= {}) as Record<string, unknown>;
}

const asNumber = (v: string): number | string => (/^-?\d+(\.\d+)?$/.test(v) ? Number(v) : v);

/** The volume name in `name:/path`, or null for bind mounts and anonymous volumes. */
function namedSource(spec: string): string | null {
  if (/^[a-zA-Z]:[\\/]/.test(spec)) return null;
  const i = spec.indexOf(':');
  if (i < 0) return null;
  const src = spec.slice(0, i);
  return NAME.test(src) ? src : null;
}

/** --mount type=bind,source=…,target=… as the long volume syntax. */
function convertMount(spec: string, ctx: Ctx): { mount: Record<string, unknown>; unknown: string[] } {
  const m: Record<string, unknown> = { type: 'volume' };
  const unknown: string[] = [];
  const sub = (key: string) => (m[key] ??= {}) as Record<string, unknown>;
  for (const part of spec.split(',')) {
    if (!part) continue;
    const eq = part.indexOf('=');
    const k = (eq < 0 ? part : part.slice(0, eq)).trim().toLowerCase();
    const v = eq < 0 ? undefined : part.slice(eq + 1);
    const on = v === undefined || !/^(false|0)$/i.test(v);
    if (k === 'type' && v) m.type = v;
    else if ((k === 'source' || k === 'src') && v !== undefined) m.source = v;
    else if ((k === 'target' || k === 'destination' || k === 'dst') && v) m.target = v;
    else if (k === 'readonly' || k === 'ro') on ? (m.read_only = true) : delete m.read_only;
    else if (k === 'bind-propagation' && v) sub('bind').propagation = v;
    else if (k === 'volume-nocopy') on && (sub('volume').nocopy = true);
    else if (k === 'volume-subpath' && v) sub('volume').subpath = v;
    else if (k === 'tmpfs-size' && v) sub('tmpfs').size = asNumber(v);
    else if (k === 'tmpfs-mode' && v && /^[0-7]+$/.test(v)) sub('tmpfs').mode = parseInt(v, 8);
    else if (k === 'consistency' && v) m.consistency = v;
    else unknown.push(part);
  }
  const ordered: Record<string, unknown> = {};
  for (const k of ['type', 'source', 'target', 'read_only', 'consistency', 'bind', 'volume', 'tmpfs']) if (k in m) ordered[k] = m[k];
  if (ordered.type === 'volume' && typeof ordered.source === 'string' && NAME.test(ordered.source)) ctx.namedVolumes.add(ordered.source);
  return { mount: ordered, unknown };
}

function convertGpus(v: string): Record<string, unknown> | null {
  const s = v.trim().replace(/^"(.*)"$/, '$1');
  const dev: Record<string, unknown> = { driver: 'nvidia' };
  if (s === 'all') dev.count = 'all';
  else if (/^\d+$/.test(s)) dev.count = Number(s);
  else if (s.startsWith('device=')) dev.device_ids = s.slice(7).split(',').filter(Boolean);
  else {
    for (const part of s.split(',')) {
      const [k, val] = part.split('=');
      if (k === 'count' && val) dev.count = val === 'all' ? 'all' : Number(val);
      else if (k === 'driver' && val) dev.driver = val;
      else if (k === 'capabilities' && val) dev.capabilities = [val];
      else return null;
    }
  }
  dev.capabilities ??= ['gpu'];
  return dev;
}

type Handler = (ctx: Ctx, v: string) => boolean | void;

const setKey =
  (key: string, map: (v: string) => unknown = (v) => v): Handler =>
  ({ svc }, v) => {
    svc[key] = map(v);
  };
const listKey =
  (key: string): Handler =>
  ({ svc }, v) =>
    push(svc, key, v);

const health =
  (key: string, map: (v: string) => unknown = (v) => v): Handler =>
  ({ svc }, v) => {
    obj(svc, 'healthcheck')[key] = map(v);
  };

/** Options that take a value. Returning false means "couldn't convert this one". */
const VALUE_HANDLERS: Record<string, Handler> = {
  name: setKey('container_name'),
  publish: listKey('ports'),
  volume: (ctx, v) => {
    push(ctx.svc, 'volumes', v);
    const named = namedSource(v);
    if (named) ctx.namedVolumes.add(named);
  },
  env: listKey('environment'),
  'env-file': listKey('env_file'),
  restart: setKey('restart'),
  network: (ctx, v) => void ctx.networks.push(v),
  'network-alias': (ctx, v) => void ctx.aliases.push(v),
  ip: (ctx, v) => void (ctx.ipv4 = v),
  ip6: (ctx, v) => void (ctx.ipv6 = v),
  hostname: setKey('hostname'),
  domainname: setKey('domainname'),
  workdir: setKey('working_dir'),
  user: setKey('user'),
  entrypoint: setKey('entrypoint', (v) => (v === '' ? [] : /\s/.test(v) ? [v] : v)),
  label: listKey('labels'),
  'label-file': listKey('label_file'),
  annotation: listKey('annotations'),
  'cap-add': listKey('cap_add'),
  'cap-drop': listKey('cap_drop'),
  device: listKey('devices'),
  'device-cgroup-rule': listKey('device_cgroup_rules'),
  tmpfs: listKey('tmpfs'),
  'shm-size': setKey('shm_size'),
  memory: setKey('mem_limit'),
  'memory-reservation': setKey('mem_reservation'),
  'memory-swap': setKey('memswap_limit'),
  'memory-swappiness': setKey('mem_swappiness', asNumber),
  'oom-score-adj': setKey('oom_score_adj', asNumber),
  'pids-limit': setKey('pids_limit', asNumber),
  cpus: setKey('cpus', asNumber),
  'cpu-shares': setKey('cpu_shares', asNumber),
  'cpu-period': setKey('cpu_period', asNumber),
  'cpu-quota': setKey('cpu_quota', asNumber),
  'cpuset-cpus': setKey('cpuset'),
  'add-host': listKey('extra_hosts'),
  dns: listKey('dns'),
  'dns-search': listKey('dns_search'),
  'dns-option': listKey('dns_opt'),
  'mac-address': setKey('mac_address'),
  link: listKey('links'),
  'volumes-from': listKey('volumes_from'),
  'health-cmd': health('test', (v) => ['CMD-SHELL', v]),
  'health-interval': health('interval'),
  'health-timeout': health('timeout'),
  'health-retries': health('retries', asNumber),
  'health-start-period': health('start_period'),
  'health-start-interval': health('start_interval'),
  'log-driver': ({ svc }, v) => void (obj(svc, 'logging').driver = v),
  'log-opt': ({ svc }, v) => {
    const eq = v.indexOf('=');
    if (eq < 1) return false;
    const logging = obj(svc, 'logging');
    ((logging.options ??= {}) as Record<string, string>)[v.slice(0, eq)] = v.slice(eq + 1);
  },
  gpus: ({ svc }, v) => {
    const dev = convertGpus(v);
    if (!dev) return false;
    svc.deploy = { resources: { reservations: { devices: [dev] } } };
  },
  platform: setKey('platform'),
  'security-opt': listKey('security_opt'),
  sysctl: listKey('sysctls'),
  ulimit: ({ svc }, v) => {
    const m = /^([a-z]+)=(-?\d+)(?::(-?\d+))?$/.exec(v);
    if (!m) return false;
    obj(svc, 'ulimits')[m[1]] = m[3] === undefined ? Number(m[2]) : { soft: Number(m[2]), hard: Number(m[3]) };
  },
  'storage-opt': ({ svc }, v) => {
    const eq = v.indexOf('=');
    if (eq < 1) return false;
    obj(svc, 'storage_opt')[v.slice(0, eq)] = v.slice(eq + 1);
  },
  expose: listKey('expose'),
  pull: setKey('pull_policy'),
  'stop-signal': setKey('stop_signal'),
  'stop-timeout': setKey('stop_grace_period', (v) => (/^\d+$/.test(v) ? `${v}s` : v)),
  ipc: setKey('ipc'),
  pid: setKey('pid'),
  uts: setKey('uts'),
  userns: setKey('userns_mode'),
  cgroupns: setKey('cgroup'),
  'cgroup-parent': setKey('cgroup_parent'),
  runtime: setKey('runtime'),
  isolation: setKey('isolation'),
  'group-add': listKey('group_add'),
};

const BOOL_KEYS: Record<string, string> = {
  init: 'init',
  privileged: 'privileged',
  'read-only': 'read_only',
  interactive: 'stdin_open',
  tty: 'tty',
  'oom-kill-disable': 'oom_kill_disable',
};

const ORDER = (
  'image container_name hostname domainname platform pull_policy command entrypoint working_dir user group_add ' +
  'restart init privileged read_only stdin_open tty ports expose volumes volumes_from tmpfs environment env_file ' +
  'labels label_file annotations network_mode networks links extra_hosts dns dns_search dns_opt mac_address cap_add ' +
  'cap_drop security_opt devices device_cgroup_rules sysctls ulimits ipc pid uts userns_mode cgroup cgroup_parent ' +
  'runtime isolation shm_size mem_limit mem_reservation memswap_limit mem_swappiness oom_kill_disable oom_score_adj ' +
  'pids_limit cpus cpu_shares cpu_period cpu_quota cpuset storage_opt healthcheck logging stop_signal ' +
  'stop_grace_period deploy'
).split(' ');
const HEALTH_ORDER = ['test', 'interval', 'timeout', 'retries', 'start_period', 'start_interval', 'disable'];

function ordered(o: Record<string, unknown>, order: string[]): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const k of order) if (k in o) out[k] = o[k];
  for (const k of Object.keys(o)) if (!(k in out)) out[k] = o[k];
  return out;
}

/** A word Compose would split back exactly as written. */
const PLAIN_WORD = /^[\w@%+=:,./-]+$/;

function serviceName(name: string | undefined, image: string): string {
  let base = name;
  if (!base) {
    const noDigest = image.split('@')[0];
    base = noDigest.slice(noDigest.lastIndexOf('/') + 1).split(':')[0];
  }
  const clean = base
    .toLowerCase()
    .replace(/[^a-z0-9_.-]+/g, '-')
    .replace(/^[^a-z0-9]+|-+$/g, '');
  return clean || 'app';
}

export interface ComposeResult {
  yaml: string;
  services: string[];
  /** Options this tool doesn't turn into Compose, as typed. */
  notConverted: string[];
  /** -d, --rm and friends, which a compose file doesn't need. */
  dropped: string[];
  /** Networks declared with external: true, because docker run needs them to exist already. */
  externalNetworks: string[];
  /** Lines that weren't docker run commands. */
  skipped: string[];
}

export function convertDockerRun(text: string): ComposeResult {
  const { commands, skipped } = splitCommands(tokenize(text));
  if (!commands.length) throw new Error('Paste a docker run command, such as docker run -d -p 8080:80 nginx.');

  const services: Record<string, Svc> = {};
  const namedVolumes = new Set<string>();
  const topNetworks = new Set<string>();
  const notConverted: string[] = [];
  const dropped = new Set<string>();

  commands.forEach((args, n) => {
    const run = parseRunArgs(args);
    if (!run.image) {
      throw new Error(
        commands.length > 1
          ? `docker run command ${n + 1} has no image. Put the image name after the options, as in docker run -p 8080:80 nginx.`
          : 'This docker run command has no image. Put the image name after the options, as in docker run -p 8080:80 nginx.',
      );
    }
    const ctx: Ctx = { svc: { image: run.image }, namedVolumes, networks: [], aliases: [], netExtras: [] };
    const missed: string[] = [];

    for (const o of run.options) {
      if (DROPPED.has(o.name)) {
        if (o.value !== false) dropped.add(o.raw.split('=')[0]);
        continue;
      }
      if (o.name === 'network-alias' || o.name === 'ip' || o.name === 'ip6') ctx.netExtras.push(o.raw);
      if (o.name === 'mount' && typeof o.value === 'string') {
        const { mount, unknown } = convertMount(o.value, ctx);
        push(ctx.svc, 'volumes', mount);
        for (const u of unknown) missed.push(`--mount option ${u}`);
        continue;
      }
      if (o.name === 'no-healthcheck') {
        if (o.value === true) ctx.svc.healthcheck = { disable: true };
        continue;
      }
      if (typeof o.value === 'boolean') {
        const key = BOOL_KEYS[o.name];
        if (key) {
          if (o.value) ctx.svc[key] = true;
        } else missed.push(o.raw);
        continue;
      }
      const h = VALUE_HANDLERS[o.name];
      if (!h || h(ctx, o.value) === false) missed.push(o.raw);
    }

    // Networks: host, none and container:x become network_mode; bridge is what Compose does anyway.
    const custom: string[] = [];
    for (const net of ctx.networks) {
      if (net === 'host' || net === 'none' || net.startsWith('container:')) ctx.svc.network_mode = net;
      else if (net !== 'bridge' && net !== 'default') custom.push(net);
    }
    if (custom.length) {
      for (const net of custom) topNetworks.add(net);
      if (ctx.aliases.length || ctx.ipv4 || ctx.ipv6) {
        const nets: Record<string, unknown> = {};
        custom.forEach((net, k) => {
          const cfg: Record<string, unknown> = {};
          if (k === 0) {
            if (ctx.aliases.length) cfg.aliases = ctx.aliases;
            if (ctx.ipv4) cfg.ipv4_address = ctx.ipv4;
            if (ctx.ipv6) cfg.ipv6_address = ctx.ipv6;
          }
          nets[net] = cfg;
        });
        ctx.svc.networks = nets;
      } else ctx.svc.networks = custom;
    } else missed.push(...ctx.netExtras);

    if (run.args.length) ctx.svc.command = run.args.every((a) => PLAIN_WORD.test(a)) ? run.args.join(' ') : run.args;
    if (ctx.svc.healthcheck) ctx.svc.healthcheck = ordered(ctx.svc.healthcheck as Record<string, unknown>, HEALTH_ORDER);

    const base = serviceName(ctx.svc.container_name as string | undefined, run.image);
    let name = base;
    for (let k = 2; name in services; k++) name = `${base}-${k}`;
    services[name] = ordered(ctx.svc, ORDER);
    notConverted.push(...missed.map((m) => (commands.length > 1 ? `${name}: ${m}` : m)));
  });

  const doc: Record<string, unknown> = { services };
  if (namedVolumes.size) doc.volumes = Object.fromEntries([...namedVolumes].map((v) => [v, {}]));
  if (topNetworks.size) doc.networks = Object.fromEntries([...topNetworks].map((n) => [n, { external: true }]));

  return {
    yaml: yaml.dump(doc, { lineWidth: -1, noRefs: true }),
    services: Object.keys(services),
    notConverted,
    dropped: [...dropped],
    externalNetworks: [...topNetworks],
    skipped,
  };
}
