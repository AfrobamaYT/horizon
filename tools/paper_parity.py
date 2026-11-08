#!/usr/bin/env python3
"""Paper event / Horizon hook traces. Build the native trace engine before `run`.

All generated data lives in .local/paper-parity. No CI or ledger updates.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import zipfile

import lab
import oracle_harness as oracle
import server_bench as bench

FIXTURES = lab.ROOT / 'tools/fixtures'
SCENARIOS = lab.ROOT / 'tools/paper_parity_scenarios.json'
HOME_KEYS = ('HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME',
             'HORIZON_LAUNCHER_TRASH', 'HORIZON_LAUNCHER_CLIPBOARD', 'HORIZON_LAUNCHER_DESKTOP')


def hook_map():
    """Read catalog attributes, including multiline attributes and Paper event lists."""
    mapping = {}
    for path in (lab.ROOT / 'crates/horizon-mod-api/src/hooks').glob('*.rs'):
        for attrs, hook in re.findall(r'#\[hook\((.*?)\)\]\s*fn\s+(\w+)\s*\(', path.read_text(), re.S):
            if match := re.search(r'\bpaper\s*=\s*("[^"]+"|\[[^\]]*\])', attrs):
                mapping[hook] = [s.strip() for v in re.findall(r'"([^"]+)"', match[1])
                                 for s in v.split(',')]
    return mapping


def chunk_fields(side, fields):
    # The sole per-hook normalization table for this slice.
    if side == 'paper':
        fields = dict(fields, dimension=fields['world'])
    else:
        fields = dict(fields, x=fields['chunk'][0], z=fields['chunk'][1])
    value = tuple(fields[k] for k in ('dimension', 'x', 'z', 'new'))
    if not (isinstance(value[0], str) and type(value[1]) is int
            and type(value[2]) is int and type(value[3]) is bool):
        raise ValueError('invalid chunk fields')
    return value


NORMALIZE = {'chunk_load': chunk_fields}


def trace(path, names, side, normalize):
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    joins = [i for i, row in enumerate(rows) if row['name'] == '_join']
    quits = [i for i, row in enumerate(rows) if row['name'] == '_quit']
    if len(joins) != 1 or len(quits) != 1 or quits[0] <= joins[0]:
        raise ValueError('missing or repeated join/quit markers')
    result = set()
    for row in rows[joins[0] + 1:quits[0]]:
        if row['name'] in names:
            if type(row['tick']) is not int or row['cancelled'] is not False:
                raise ValueError('invalid tick or cancellation on listen hook')
            result.add(normalize(side, row['fields']))
    return result


def report(out):
    manifest = json.loads((out / 'run.json').read_text())
    mapping, failed = hook_map(), False
    for hook, events in sorted(mapping.items()):
        if hook not in manifest['scenario']['hooks'] or hook not in NORMALIZE:
            print(f'{hook}: not covered')
            continue
        try:
            if manifest.get('error') or not manifest.get('complete'):
                raise ValueError(manifest.get('error', 'incomplete run'))
            left = trace(out / 'paper.jsonl', events, 'paper', NORMALIZE[hook])
            right = trace(out / 'horizon.jsonl', [hook], 'horizon', NORMALIZE[hook])
            if not left or not right:
                raise ValueError('empty observation window')
            if left != right:
                raise ValueError(f'Paper-only {len(left - right)} {sorted(left - right)[:3]}; '
                                 f'Horizon-only {len(right - left)} {sorted(right - left)[:3]}')
            print(f'{hook}: match ({len(left)} chunks; set, tick/order ignored)')
        except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
            failed = True
            print(f'{hook}: mismatch: {error}')
    if any(h not in mapping for h in manifest['scenario']['hooks']):
        print('mismatch: scenario hook missing from catalog')
        failed = True
    return int(failed)


def environment(directory):
    env = os.environ.copy()
    for key in HOME_KEYS:
        env[key] = tempfile.mkdtemp(prefix=key.lower() + '-', dir=directory)
    return lab.game_env(env)


def properties(directory, scenario, port):
    directory.mkdir()
    props = dict(bench.PROPERTIES, **{'server-ip': '127.0.0.1', 'server-port': port,
        'level-seed': scenario['seed'], 'level-name': 'world', 'level-type': 'minecraft\\:flat',
        'view-distance': 2, 'simulation-distance': 2, 'generate-structures': 'false',
        'allow-flight': 'true', 'pause-when-empty-seconds': -1})
    (directory / 'server.properties').write_text(''.join(f'{k}={v}\n' for k, v in props.items()))
    (directory / 'eula.txt').write_text('eula=true\n')


def compile_plugin(out, jar):
    # Paperclip is a bootstrap jar: its first start populates the cached API/dependency jars.
    shared = lab.LOCAL / 'server-bench/paper-shared'
    dependencies = os.pathsep.join(map(str, [jar, *sorted(shared.rglob('*.jar'))]))
    classes = out / 'classes'
    classes.mkdir()
    oracle.run_logged(['javac', '--release', str(lab.config()[0]['minecraft']['java_major']),
        '-proc:none', '-cp', dependencies, '-d', classes,
        FIXTURES / 'paper-trace-plugin/PaperTrace.java'], out, 'compile-java-PaperTrace')
    plugin = out / 'paper-trace.jar'
    with zipfile.ZipFile(plugin, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.write(FIXTURES / 'paper-trace-plugin/plugin.yml', 'plugin.yml')
        for path in classes.rglob('*.class'):
            archive.write(path, path.relative_to(classes).as_posix())
    return plugin


def run(args):
    scenario = json.loads(SCENARIOS.read_text())['chunk_load']
    out = Path(tempfile.mkdtemp(prefix='chunk-load-', dir=lab.LOCAL / 'paper-parity'))
    manifest = {'schema': 1, 'scenario': scenario, 'complete': False,
                'bot_uuid': str(lab.offline_uuid('Bot0'))}
    lab.write_json(out / 'run.json', manifest)
    try:
        installed = args.engine.parent / 'horizon-native-mods/paper_trace/src/lib.rs'
        if installed.read_bytes() != (FIXTURES / 'paper-trace-mod/src/lib.rs').read_bytes():
            raise ValueError('export does not contain the current paper_trace fixture; rebuild it')
        # Reuse PaperMC's checksum-verified downloader and the pinned Mojang cache setup.
        boot = out / 'bootstrap'
        properties(boot, scenario, lab.free_port())
        paper = bench.server_command('paper', boot, SimpleNamespace(heap=args.heap))
        jar = Path(paper[paper.index('-jar') + 1])
        manifest['paper_jar_sha256'] = lab.digest(jar, 'sha256')
        manifest['engine_sha256'] = lab.digest(args.engine, 'sha256')
        server = bench.Server(paper, boot, out / 'bootstrap.log', environment(boot))
        try:
            server.wait_for(bench.DONE, 600)
        finally:
            server.stop()
        plugin = compile_plugin(out, jar)
        spawns = []
        for side in ('paper', 'horizon'):
            directory, port = out / side, lab.free_port()
            properties(directory, scenario, port)
            env = environment(directory)
            trace_path = out / f'{side}.jsonl'
            if side == 'paper':
                for name in ('cache', 'libraries', 'versions'):
                    lab.link_dir(lab.LOCAL / 'server-bench/paper-shared' / name, directory / name)
                (directory / 'bukkit.yml').write_text('settings:\n  connection-throttle: -1\n')
                (directory / 'plugins').mkdir()
                shutil.copyfile(plugin, directory / 'plugins/paper-trace.jar')
                command = [paper[0], f'-Dpaper.trace={trace_path}', *paper[1:]]
            else:
                env['HORIZON_PAPER_TRACE'] = str(trace_path)
                command = [args.engine, '--dedicated', '--nogui', '--game-dir', directory,
                           '--resources', args.resources, '--world', 'world']
            server = bench.Server(command, directory, out / f'{side}.log', env)
            try:
                server.wait_for(bench.DONE, 600)
                for key, value in scenario['gamerules'].items():
                    server.console(f'gamerule {key} {str(value).lower()}')
                for command in scenario['setup']:
                    server.console(command)
                server.console('say PAPER_PARITY_SETUP_DONE')
                server.wait_for('PAPER_PARITY_SETUP_DONE', 30)
                bot = scenario['bot']
                summary = out / f'{side}-bots.json'
                oracle.run_logged([args.bots, '--server', f'127.0.0.1:{port}', '--bots', '1',
                    '--behaviour', bot['behaviour'], '--seed', str(scenario['seed']),
                    '--view-distance', '2', '--height', str(bot['height']),
                    '--explore-speed', str(bot['speed']), '--explore-distance', str(bot['distance']),
                    '--seconds', str(scenario['duration']), '--report', summary], out, f'{side}-bots',
                    timeout=scenario['duration'] + 180, env=env)
                result = json.loads(summary.read_text())
                if (result['joined'] != 1 or result['pulled_back'] or
                        any(b['error'] or not b['explore_complete'] for b in result['per_bot'])):
                    raise ValueError(f'{side}: bot failed, corrected, or did not finish path')
                spawn = [json.loads(l)['spawn'] for l in (out / f'{side}-bots.log').read_text().splitlines()
                         if l.startswith('{"spawn":')]
                if len(spawn) != 1:
                    raise ValueError(f'{side}: missing spawn position')
                spawns.append(spawn[0])
                deadline = time.monotonic() + 30
                while not trace_path.exists() or '"_quit"' not in trace_path.read_text():
                    if time.monotonic() >= deadline:
                        raise ValueError(f'{side}: no quit marker (trace mod/plugin missing?)')
                    time.sleep(0.1)
            finally:
                server.stop()
        if spawns[0] != spawns[1]:
            raise ValueError(f'different path origins: {spawns}')
        manifest['complete'] = True
    except Exception as error:
        manifest['error'] = str(error)
    lab.write_json(out / 'run.json', manifest)
    print(f'Traces and logs: {out}')
    return report(out)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    start = commands.add_parser('run')
    start.add_argument('--engine', type=Path, required=True, help='export containing paper_trace')
    start.add_argument('--resources', type=Path, default=lab.RESOURCES)
    start.add_argument('--bots', type=Path, default=lab.cargo_target() / f'fast/horizon-bots{bench.EXE}')
    start.add_argument('--heap', default='2g')
    diff = commands.add_parser('report')
    diff.add_argument('out', type=Path)
    args = parser.parse_args()
    if args.command == 'report':
        return report(args.out.resolve())
    for key in ('engine', 'resources', 'bots'):
        setattr(args, key, getattr(args, key).resolve())
    cache = lab.LOCAL / 'paper-parity'
    cache.mkdir(parents=True, exist_ok=True)
    # tempfile, Java and tool subprocesses must also stay beside the worktree.
    scratch = cache / 'tmp'
    scratch.mkdir(exist_ok=True)
    tempfile.tempdir = str(scratch)
    os.environ.update(TEMP=str(scratch), TMP=str(scratch), TMPDIR=str(scratch))
    return run(args)


if __name__ == '__main__':
    sys.exit(main())
