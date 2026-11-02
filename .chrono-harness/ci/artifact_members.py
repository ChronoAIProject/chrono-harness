"""Acquire one explicitly addressed artifact member using Python's ZIP reader.

The caller owns typed inputs, logical addresses, admission, process/stream bounds
and the single artifact deadline. No descriptor discovery or full ZIP fallback.
The range capability is adapted from the admitted collect_originals.py reader;
ZIP interpretation, decompression and CRC validation remain with stdlib zipfile.
"""
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


class Deadline:
    def __init__(self, seconds, clock=time.monotonic):
        self.clock = clock
        self.end = clock() + seconds

    def remaining(self):
        remaining = self.end - self.clock()
        if remaining <= 0:
            raise TimeoutError('artifact acquisition deadline exhausted')
        return remaining


class RangedZipFile(io.RawIOBase):
    def __init__(self, url, length, deadline, open_url=urllib.request.urlopen):
        self.url, self.length, self.deadline = url, length, deadline
        self.open_url = open_url
        self.position = 0
        self.requests = 0
        self.payload_bytes = 0

    def readable(self):
        return True

    def seekable(self):
        return True

    def tell(self):
        return self.position

    def seek(self, offset, whence=0):
        self.deadline.remaining()
        if whence not in (0, 1, 2):
            raise ValueError('unsupported ZIP seek')
        position = offset if whence == 0 else (self.position if whence == 1 else self.length) + offset
        if position < 0:
            raise ValueError('negative ZIP seek')
        self.position = position
        return position

    def read(self, count=-1):
        self.deadline.remaining()
        if count < 0:
            count = self.length - self.position
        count = min(count, max(0, self.length - self.position))
        if count <= 0:
            return b''
        # Match the existing bounded reader. No increase for ZIP metadata.
        if count > 64 * 1024 * 1024:
            raise ValueError('ZIP read exceeds original reader limit')
        first, last = self.position, self.position + count - 1
        request = urllib.request.Request(self.url, headers={
            'Range': 'bytes=%s-%s' % (first, last), 'Accept-Encoding': 'identity'})
        self.requests += 1
        # Signed archive requests have no API Authorization header. Redirects
        # are rejected: the admitted archive location must supply these ranges.
        with self.open_url(request, timeout=self.deadline.remaining()) as response:
            expected = 'bytes %s-%s/%s' % (first, last, self.length)
            if response.status != 206 or response.headers.get('Content-Range') != expected:
                raise ValueError('ZIP range response identity mismatch')
            if response.headers.get('Content-Encoding', 'identity') != 'identity':
                raise ValueError('ZIP range response encoding mismatch')
            data = bytearray()
            # Check the cumulative deadline even if the server keeps a socket
            # alive with short reads. The supervising process shares this end.
            while len(data) <= count:
                self.deadline.remaining()
                chunk = response.read(min(131072, count + 1 - len(data)))
                if not chunk:
                    break
                data.extend(chunk)
        self.deadline.remaining()
        self.payload_bytes += len(data)
        if len(data) != count:
            raise ValueError('ZIP range size mismatch')
        self.position += count
        return bytes(data)


def member_name(value):
    path = PurePosixPath(value)
    if (not value or path.is_absolute() or '..' in path.parts or
            '\\' in value or '\x00' in value or path.as_posix() != value):
        raise ValueError('invalid declared ZIP member address')
    return value


def acquire(remote, name, limit):
    """Only the caller-declared name can become a local input."""
    member_name(name)
    with zipfile.ZipFile(remote) as archive:
        matches = [info for info in archive.infolist() if info.filename == name]
        if len(matches) != 1:
            raise ValueError('declared ZIP member missing or ambiguous')
        info = matches[0]
        if info.is_dir() or ((info.external_attr >> 16) & 0o170000) not in (0, 0o100000):
            raise ValueError('declared ZIP member is not a regular file')
        if info.file_size > limit or info.flag_bits & 1:
            raise ValueError('declared ZIP member exceeds limit or is encrypted')
        with archive.open(info) as source:
            result = bytearray()
            while len(result) <= limit:
                remote.deadline.remaining()
                chunk = source.read(min(131072, limit + 1 - len(result)))
                if not chunk:
                    break
                result.extend(chunk)
        remote.deadline.remaining()
        if len(result) != info.file_size or len(result) > limit:
            raise ValueError('declared ZIP member length mismatch')
        return bytes(result)


def signed_archive(url, deadline, token):
    parsed = urllib.parse.urlsplit(url)
    if (parsed.scheme != 'https' or parsed.netloc != 'api.github.com' or
            not re.fullmatch(r'/repos/[^/]+/[^/]+/actions/artifacts/[0-9]+/zip', parsed.path) or
            parsed.query or parsed.fragment):
        raise ValueError('unsupported artifact archive endpoint')
    request = urllib.request.Request(url, headers={
        'Authorization': 'Bearer ' + token, 'Accept': 'application/vnd.github+json'})
    try:
        with urllib.request.build_opener(NoRedirect()).open(request, timeout=deadline.remaining()):
            raise ValueError('artifact archive API did not redirect')
    except urllib.error.HTTPError as error:
        if error.code != 302:
            raise
        location = error.headers.get('Location', '')
        target = urllib.parse.urlsplit(location)
        if target.scheme != 'https' or not target.netloc or target.username or target.password:
            raise ValueError('invalid artifact archive redirect')
        return location


def main(request):
    fields = {'schema', 'archive_url', 'archive_length', 'member', 'destination', 'limit', 'remaining_seconds'}
    if not isinstance(request, dict) or set(request) != fields or request['schema'] != 'chrono-artifact-member-request/v1':
        raise ValueError('unsupported typed artifact byte request')
    url, length, name, destination, limit, seconds = (request[field] for field in
        ['archive_url', 'archive_length', 'member', 'destination', 'limit', 'remaining_seconds'])
    length, limit, seconds = int(length), int(limit), float(seconds)
    if length <= 0 or not 0 <= limit <= 64 * 1024 * 1024 or not 0 < seconds <= 60:
        raise ValueError('invalid artifact acquisition bounds')
    deadline = Deadline(seconds)
    token = os.environ.get('GH_TOKEN')
    if not token:
        raise ValueError('declared GH_TOKEN input missing')
    location = signed_archive(url, deadline, token)
    remote = RangedZipFile(location, length, deadline,
                           urllib.request.build_opener(NoRedirect()).open)
    raw = acquire(remote, name, limit)
    deadline.remaining()
    target = Path(destination)
    target.parent.mkdir(parents=True, exist_ok=True)
    # A failed process leaves preserved bytes in its isolated acquisition;
    # only a complete successful process can be consumed by the caller.
    with target.open('xb') as output:
        output.write(raw)
    deadline.remaining()
    print(json.dumps({'schema': 'chrono-artifact-member/v1', 'member': name,
                      'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(),
                      'range_requests': remote.requests,
                      'acquired_payload_bytes': remote.payload_bytes,
                      'archive_api_requests': 1, 'full_archive_verified': False}))


if __name__ == '__main__':
    try:
        if len(sys.argv) != 1:
            raise ValueError('artifact member transport accepts only its declared stdin request')
        main(json.load(sys.stdin))
    except Exception as error:
        # Preserve the original error class/status/message without persisting
        # redirect capabilities or credential values in process observations.
        message = re.sub(r'https?://[^\s\x27\x22]+', '[URL withheld]', str(error))
        token = os.environ.get('GH_TOKEN')
        if token:
            message = message.replace(token, '[credential withheld]')
        if isinstance(error, urllib.error.HTTPError):
            message = 'HTTP %s: %s' % (error.code, message)
        print(json.dumps({'error_type': type(error).__name__, 'message': message}), file=sys.stderr)
        sys.exit(2)
