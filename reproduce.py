"""Reproduce the original executable. No keys, signing, deployment or RPC writes."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parent
EXPECTED = '5424ba2462eb78826c7a4e7f8dacc2d11fc8cffc1bcff8ac33db215db4f132fb'
ORIGINAL_VENDOR = (
    r'C:\Users\peped\Documents\Codex\2026-09-09\referenced-chatgpt-conversation-this-is-an'
    r'\work\solana-mini-launch-2026-09-21\launch-program\..\fee-adapter\vendor'
)
RELEASES = {
    'win32': ('windows-x86_64', '792bc821f006e2b56aee31640110b449d5dc89f1305f16f24037465ab8cd7d5a'),
    'linux': ('linux-x86_64', 'b0f7af104adf726fff2a6a09ea2eb2f2d2965c92295f4d7388c08d140e0c2b00'),
}

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def download(url, dest, expected):
    if dest.is_file() and digest(dest) == expected:
        return
    temp = dest.with_suffix(dest.suffix + '.part')
    request = urllib.request.Request(url, headers={'User-Agent': 'ElTrueque-reproducible-build'})
    with urllib.request.urlopen(request, timeout=120) as source, temp.open('wb') as output:
        shutil.copyfileobj(source, output)
    if digest(temp) != expected:
        raise RuntimeError(f'Checksum mismatch for {dest.name}')
    temp.replace(dest)

def extract(archive, destination):
    with tarfile.open(archive) as source:
        source.extractall(destination, filter='data')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    choice = parser.add_mutually_exclusive_group(required=True)
    choice.add_argument('--platform-tools', type=Path)
    choice.add_argument('--download-tools', action='store_true')
    parser.add_argument('--vendor-dir', type=Path, help='Use existing checksum-verified crate sources')
    args = parser.parse_args()
    work = ROOT/'.build'
    work.mkdir(exist_ok=True)
    extension = '.exe' if sys.platform == 'win32' else ''
    if args.download_tools:
        if sys.platform not in RELEASES:
            raise RuntimeError('Pinned downloads support Windows x86_64 and Linux x86_64')
        system, checksum = RELEASES[sys.platform]
        filename = f'platform-tools-{system}.tar.bz2'
        archive = work/filename
        download(f'https://github.com/anza-xyz/platform-tools/releases/download/v1.57/{filename}', archive, checksum)
        tools = work/'platform-tools'
        if not (tools/f'rust/bin/rustc{extension}').is_file():
            tools.mkdir(exist_ok=True)
            extract(archive, tools)
    else:
        tools = args.platform_tools.resolve()
    rustc = tools/f'rust/bin/rustc{extension}'
    compiler_version = subprocess.check_output([str(rustc), '--version'], text=True).strip()
    if compiler_version != 'rustc 1.95.0-dev (ae660768a 2026-08-17)':
        raise RuntimeError(f'Unexpected compiler: {compiler_version}')
    vendor = args.vendor_dir.resolve() if args.vendor_dir else work/'vendor'
    vendor.mkdir(exist_ok=True)
    lock = tomllib.loads((ROOT/'program/Cargo.lock').read_text(encoding='utf8'))
    for package in lock['package']:
        if 'source' not in package:
            continue
        name, version = package['name'], package['version']
        folder = vendor/f'{name}-{version}'
        checksum_file = folder/'.cargo-checksum.json'
        if not folder.exists():
            archive = work/f'{name}-{version}.crate'
            download(f'https://static.crates.io/crates/{name}/{name}-{version}.crate', archive, package['checksum'])
            extract(archive, vendor)
            files = {p.relative_to(folder).as_posix(): digest(p) for p in folder.rglob('*') if p.is_file()}
            checksum_file.write_text(json.dumps({'package': package['checksum'], 'files': files}), encoding='utf8')
        checks = json.loads(checksum_file.read_text(encoding='utf8'))
        if checks['package'] != package['checksum']:
            raise RuntimeError(f'Package checksum mismatch: {name}')
        for relative, expected in checks['files'].items():
            if digest(folder/relative) != expected:
                raise RuntimeError(f'Source checksum mismatch: {name}/{relative}')
    flags = ['-C', 'panic=abort', '-C', 'target-cpu=v3']
    # Map whole filenames so Windows diagnostic separators remain identical on Linux.
    for source in sorted(vendor.rglob('*.rs')):
        suffix = str(source.relative_to(vendor)).replace('/', '\\')
        flags.append(f'--remap-path-prefix={source}={ORIGINAL_VENDOR}\\{suffix}')
    env = dict(os.environ)
    env.pop('RUSTFLAGS', None)
    env['CARGO_ENCODED_RUSTFLAGS'] = '\x1f'.join(flags)
    env['RUSTC'] = str(rustc)
    env['CARGO_HOME'] = str(work/'cargo-home')
    env['CARGO_TARGET_DIR'] = str(work/'target')
    env['PATH'] = str(tools/'rust/bin') + os.pathsep + str(tools/'llvm/bin') + os.pathsep + env.get('PATH', '')
    command = [str(tools/f'rust/bin/cargo{extension}'), 'build', '--offline', '--release', '--locked',
               '--target', 'sbpfv3-solana-solana', '--config', 'source.crates-io.replace-with="vendored-sources"',
               '--config', f'source.vendored-sources.directory="{vendor.as_posix()}"']
    subprocess.run(command, cwd=ROOT/'program', env=env, check=True)
    binary = work/'target/sbpfv3-solana-solana/release/mini_launch_solana.so'
    actual = digest(binary)
    result = {'expectedSha256': EXPECTED, 'actualSha256': actual, 'exactMatch': actual == EXPECTED,
              'bytes': binary.stat().st_size, 'host': sys.platform, 'compiler': compiler_version}
    (work/'result.json').write_text(json.dumps(result, indent=2)+'\n', encoding='utf8')
    print(json.dumps(result, indent=2))
    if actual != EXPECTED:
        raise SystemExit('Build mismatch: public verification must not be claimed')

if __name__ == '__main__':
    main()
