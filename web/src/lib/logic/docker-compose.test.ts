import { describe, expect, it } from 'vitest';
import yaml from 'js-yaml';
import { convertDockerRun, splitCommands, tokenize } from './docker-compose';

/* eslint-disable @typescript-eslint/no-explicit-any */
const convert = (text: string) => {
  const r = convertDockerRun(text);
  return { ...r, doc: yaml.load(r.yaml) as any };
};
const service = (text: string) => {
  const { doc } = convert(text);
  return Object.values(doc.services)[0] as any;
};

describe('tokenize', () => {
  it('splits on whitespace and honours quotes', () => {
    expect(tokenize(`docker run -e "A=hello world" -e 'B=it''s' nginx`).flat()).toEqual([
      'docker', 'run', '-e', 'A=hello world', '-e', 'B=its', 'nginx',
    ]);
  });

  it('joins bash, cmd and PowerShell line continuations', () => {
    for (const c of ['\\', '^', '`']) {
      expect(tokenize(`docker run ${c}\n  -p 80:80 ${c}\r\n  nginx`)).toEqual([['docker', 'run', '-p', '80:80', 'nginx']]);
    }
  });

  it('keeps Windows paths and handles escapes', () => {
    expect(tokenize('docker run -v C:\\data:/data a\\ b "say \\"hi\\""')).toEqual([
      ['docker', 'run', '-v', 'C:\\data:/data', 'a b', 'say "hi"'],
    ]);
  });

  it('escapes dollar signs that the shell would have kept literal', () => {
    expect(tokenize(`docker run -e 'P=a$b' -e "H=$HOME" -e Q=\\$x img`)).toEqual([
      ['docker', 'run', '-e', 'P=a$$b', '-e', 'H=$HOME', '-e', 'Q=$$x', 'img'],
    ]);
  });

  it('separates commands on new lines, ; and &&, and skips comments', () => {
    expect(tokenize('# start\ndocker run a && docker run b; docker run c # trailing\n\n')).toEqual([
      ['docker', 'run', 'a'],
      ['docker', 'run', 'b'],
      ['docker', 'run', 'c'],
    ]);
  });

  it('reports an unclosed quote', () => {
    expect(() => tokenize('docker run -e "A=1 nginx')).toThrow(/quote/);
  });
});

describe('splitCommands', () => {
  it('finds docker run, docker container run, sudo and podman', () => {
    const r = splitCommands(tokenize('sudo docker run a\ndocker container run b\npodman run c\nls -la'));
    expect(r.commands).toEqual([['a'], ['b'], ['c']]);
    expect(r.skipped).toEqual(['ls -la']);
  });
});

describe('convertDockerRun: basics', () => {
  it('converts a typical command', () => {
    const { doc, notConverted, dropped } = convert(
      'docker run -d --rm --name web -p 8080:80 -v ./site:/usr/share/nginx/html:ro -e TZ=Pacific/Auckland --restart unless-stopped nginx:1.27',
    );
    expect(doc).toEqual({
      services: {
        web: {
          image: 'nginx:1.27',
          container_name: 'web',
          restart: 'unless-stopped',
          ports: ['8080:80'],
          volumes: ['./site:/usr/share/nginx/html:ro'],
          environment: ['TZ=Pacific/Auckland'],
        },
      },
    });
    expect(notConverted).toEqual([]);
    expect(dropped).toEqual(['-d', '--rm']);
  });

  it('puts image first and names the service after the image when there is no --name', () => {
    const r = convert('docker run ghcr.io/acme/my_app:2.1@sha256:abc');
    expect(Object.keys(r.doc.services)).toEqual(['my_app']);
    expect(r.yaml.split('\n')[2].trim()).toBe('image: ghcr.io/acme/my_app:2.1@sha256:abc');
  });

  it('maps the arguments after the image to command', () => {
    expect(service('docker run alpine echo hello').command).toBe('echo hello');
    expect(service('docker run alpine sh -c "echo hi && sleep 1"').command).toEqual(['sh', '-c', 'echo hi && sleep 1']);
    // Flags after the image belong to the command.
    expect(service('docker run redis redis-server --appendonly yes').command).toBe('redis-server --appendonly yes');
  });

  it('accepts --flag=value, -pVALUE and combined short flags', () => {
    const s = service('docker run -dit --name=box -p8080:80 --publish=443:443 -itp 53:53/udp busybox');
    expect(s.stdin_open).toBe(true);
    expect(s.tty).toBe(true);
    expect(s.ports).toEqual(['8080:80', '443:443', '53:53/udp']);
    expect(s.container_name).toBe('box');
  });

  it('honours boolean flags set to false', () => {
    const s = service('docker run --privileged=false --init=true alpine');
    expect(s.privileged).toBeUndefined();
    expect(s.init).toBe(true);
  });

  it('converts several commands into one file with unique service names', () => {
    const { doc } = convert('docker run -d nginx\ndocker run -d nginx\ndocker run -d --name db postgres:16');
    expect(Object.keys(doc.services)).toEqual(['nginx', 'nginx-2', 'db']);
  });

  it('sanitises the service name', () => {
    expect(Object.keys(convert('docker run --name "My App!" alpine').doc.services)).toEqual(['my-app']);
  });
});

describe('convertDockerRun: volumes and mounts', () => {
  it('declares named volumes at the top level', () => {
    const { doc } = convert('docker run -v pgdata:/var/lib/postgresql/data -v /srv/x:/x -v C:\\data:/data -v /anon postgres');
    expect(doc.services.postgres.volumes).toEqual(['pgdata:/var/lib/postgresql/data', '/srv/x:/x', 'C:\\data:/data', '/anon']);
    expect(doc.volumes).toEqual({ pgdata: {} });
  });

  it('converts --mount to the long syntax', () => {
    const { doc, notConverted } = convert(
      'docker run --mount type=bind,source=/src,target=/app,readonly ' +
        '--mount source=cache,target=/cache,volume-nocopy ' +
        '--mount type=tmpfs,destination=/tmp,tmpfs-size=64m,tmpfs-mode=1770 ' +
        '--mount type=bind,src=/a,dst=/b,bind-propagation=rshared,weird=1 node',
    );
    expect(doc.services.node.volumes).toEqual([
      { type: 'bind', source: '/src', target: '/app', read_only: true },
      { type: 'volume', source: 'cache', target: '/cache', volume: { nocopy: true } },
      { type: 'tmpfs', target: '/tmp', tmpfs: { size: '64m', mode: 0o1770 } },
      { type: 'bind', source: '/a', target: '/b', bind: { propagation: 'rshared' } },
    ]);
    expect(doc.volumes).toEqual({ cache: {} });
    expect(notConverted).toEqual(['--mount option weird=1']);
  });
});

describe('convertDockerRun: networking', () => {
  it('declares custom networks as external', () => {
    const { doc, externalNetworks } = convert(
      'docker run --network backend --network-alias api --name a img\ndocker run --net=frontend --ip 172.20.0.5 --name b img',
    );
    expect(externalNetworks).toEqual(['backend', 'frontend']);
    expect(doc.services.a.networks).toEqual({ backend: { aliases: ['api'] } });
    expect(doc.services.b.networks).toEqual({ frontend: { ipv4_address: '172.20.0.5' } });
    expect(doc.networks).toEqual({ backend: { external: true }, frontend: { external: true } });
  });

  it('uses a plain list when there are no per-network settings', () => {
    const { doc } = convert('docker run --network backend img');
    expect(doc.services.img.networks).toEqual(['backend']);
  });

  it('maps host, none and container networking to network_mode and drops bridge', () => {
    expect(service('docker run --network host img').network_mode).toBe('host');
    expect(service('docker run --net none img').network_mode).toBe('none');
    expect(service('docker run --network container:vpn img').network_mode).toBe('container:vpn');
    const r = convert('docker run --network bridge img');
    expect(r.doc.services.img.networks).toBeUndefined();
    expect(r.doc.networks).toBeUndefined();
    expect(r.notConverted).toEqual([]);
  });

  it('converts hosts, DNS and exposed ports', () => {
    const s = service('docker run --add-host db:10.0.0.2 --dns 1.1.1.1 --dns 8.8.8.8 --expose 9000 --hostname box img');
    expect(s.extra_hosts).toEqual(['db:10.0.0.2']);
    expect(s.dns).toEqual(['1.1.1.1', '8.8.8.8']);
    expect(s.expose).toEqual(['9000']);
    expect(s.hostname).toBe('box');
  });
});

describe('convertDockerRun: everything else', () => {
  it('converts the long tail of flags', () => {
    const s = service(
      [
        'docker run',
        '--env-file .env -l traefik.enable=true --label app=x',
        '-w /app -u 1000:1000 --entrypoint /bin/sh',
        '--cap-add NET_ADMIN --cap-drop ALL --device /dev/snd',
        '--privileged --read-only --tmpfs /run:size=64m --shm-size 1g',
        '-m 512m --cpus 1.5 --init --platform linux/amd64',
        '--security-opt no-new-privileges --sysctl net.core.somaxconn=1024',
        '--ulimit nofile=1024:2048 --ulimit nproc=512',
        '--pull always --stop-signal SIGINT --stop-timeout 30',
        'img',
      ].join(' '),
    );
    expect(s).toMatchObject({
      env_file: ['.env'],
      labels: ['traefik.enable=true', 'app=x'],
      working_dir: '/app',
      user: '1000:1000',
      entrypoint: '/bin/sh',
      cap_add: ['NET_ADMIN'],
      cap_drop: ['ALL'],
      devices: ['/dev/snd'],
      privileged: true,
      read_only: true,
      tmpfs: ['/run:size=64m'],
      shm_size: '1g',
      mem_limit: '512m',
      cpus: 1.5,
      init: true,
      platform: 'linux/amd64',
      security_opt: ['no-new-privileges'],
      sysctls: ['net.core.somaxconn=1024'],
      ulimits: { nofile: { soft: 1024, hard: 2048 }, nproc: 512 },
      pull_policy: 'always',
      stop_signal: 'SIGINT',
      stop_grace_period: '30s',
    });
  });

  it('builds a healthcheck', () => {
    const s = service(
      'docker run --health-cmd "curl -f http://localhost/ || exit 1" --health-interval 30s --health-timeout 5s --health-retries 3 --health-start-period 10s img',
    );
    expect(s.healthcheck).toEqual({
      test: ['CMD-SHELL', 'curl -f http://localhost/ || exit 1'],
      interval: '30s',
      timeout: '5s',
      retries: 3,
      start_period: '10s',
    });
    expect(service('docker run --no-healthcheck img').healthcheck).toEqual({ disable: true });
  });

  it('builds logging options', () => {
    const s = service('docker run --log-driver json-file --log-opt max-size=10m --log-opt max-file=3 img');
    expect(s.logging).toEqual({ driver: 'json-file', options: { 'max-size': '10m', 'max-file': '3' } });
  });

  it('reserves GPUs', () => {
    const dev = (cmd: string) => service(cmd).deploy.resources.reservations.devices;
    expect(dev('docker run --gpus all img')).toEqual([{ driver: 'nvidia', count: 'all', capabilities: ['gpu'] }]);
    expect(dev('docker run --gpus 2 img')).toEqual([{ driver: 'nvidia', count: 2, capabilities: ['gpu'] }]);
    expect(dev(`docker run --gpus '"device=0,1"' img`)).toEqual([{ driver: 'nvidia', device_ids: ['0', '1'], capabilities: ['gpu'] }]);
  });

  it('uses a list entrypoint when it has spaces, and [] to clear it', () => {
    expect(service('docker run --entrypoint "" img').entrypoint).toEqual([]);
    expect(service('docker run --entrypoint "/my app" img').entrypoint).toEqual(['/my app']);
  });

  it('lists flags it does not convert instead of dropping them', () => {
    const r = convert('docker run --cidfile /tmp/id --sig-proxy=false -P --frobnicate img');
    expect(r.notConverted).toEqual(['--cidfile /tmp/id', '--sig-proxy=false', '-P', '--frobnicate']);
    expect(r.doc.services.img.image).toBe('img');
  });

  it('quotes values that YAML would misread', () => {
    const r = convertDockerRun('docker run -p 2222:22 -e DEBUG=yes img');
    expect(r.yaml).toContain("- '2222:22'");
  });
});

describe('convertDockerRun: errors and skipped lines', () => {
  it('asks for a docker run command', () => {
    expect(() => convertDockerRun('ls -la')).toThrow(/docker run/);
  });

  it('asks for an image', () => {
    expect(() => convertDockerRun('docker run -d -p 80:80')).toThrow(/image/);
  });

  it('says which flag needs a value', () => {
    expect(() => convertDockerRun('docker run nginx -p') ).not.toThrow();
    expect(() => convertDockerRun('docker run -p')).toThrow(/-p needs a value/);
  });

  it('mentions lines that are not docker run commands', () => {
    const r = convertDockerRun('docker run nginx\n-p 80:80');
    expect(r.skipped).toEqual(['-p 80:80']);
  });
});
