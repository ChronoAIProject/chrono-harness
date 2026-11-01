"""Behavior tests for the paired artifact member acquisition implementation."""
import io
from pathlib import Path
import re
import unittest
import warnings
import zipfile

import artifact_members as product

FIXTURE = Path(__file__).parent / 'fixtures' / 'artifact-member.bin'


def archive(body, name='explicit-extra/opaque-original.raw', duplicate=False):
    target = io.BytesIO()
    with zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as output:
        output.writestr('unrequested/custody-source.raw', b'preserved wider original' * 10000)
        output.writestr(name, body)
        if duplicate:
            with warnings.catch_warnings():
                warnings.simplefilter('ignore', UserWarning)
                output.writestr(name, body)
    return target.getvalue()


class Response(io.BytesIO):
    status = 206

    def __init__(self, raw, content_range, encoding='identity'):
        super().__init__(raw)
        self.headers = {'Content-Range': content_range, 'Content-Encoding': encoding}


class Source:
    def __init__(self, body, mode='correct', tick=None):
        self.body, self.mode, self.tick = body, mode, tick
        self.requests = []

    def open(self, request, timeout):
        self.assert_request(request, timeout)
        if self.tick:
            self.tick()
        first, last = map(int, re.fullmatch(r'bytes=(\d+)-(\d+)', request.get_header('Range')).groups())
        raw = self.body[first:last + 1]
        expected = 'bytes %s-%s/%s' % (first, last, len(self.body))
        if self.mode == 'wrong-range':
            expected = 'bytes %s-%s/%s' % (first + 1, last + 1, len(self.body))
        if self.mode == 'short':
            raw = raw[:-1]
        if self.mode == 'overlong':
            raw += b'x'
        response = Response(raw, expected, 'gzip' if self.mode == 'encoded' else 'identity')
        if self.mode == 'whole-response':
            response.status = 200
        return response

    def assert_request(self, request, timeout):
        self.requests.append(request.get_header('Range'))
        if request.get_header('Authorization') is not None or timeout <= 0:
            raise AssertionError('invalid signed-archive request')


class Acquisition(unittest.TestCase):
    def remote(self, body, mode='correct', deadline=None, tick=None):
        source = Source(body, mode, tick)
        return product.RangedZipFile('https://archive.invalid/signed-capability', len(body),
                                     deadline or product.Deadline(60), source.open), source

    def test_declared_binary_original_matches_complete_zip_without_discovery(self):
        raw = FIXTURE.read_bytes()
        body = archive(raw)
        remote, source = self.remote(body)
        result = product.acquire(remote, 'explicit-extra/opaque-original.raw', len(raw))
        with zipfile.ZipFile(io.BytesIO(body)) as complete:
            self.assertEqual(result, complete.read('explicit-extra/opaque-original.raw'))
            self.assertEqual(complete.read('unrequested/custody-source.raw'), b'preserved wider original' * 10000)
        self.assertEqual(result, raw)
        self.assertTrue(source.requests)
        self.assertEqual(remote.payload_bytes, sum(int(r.split('-')[1]) - int(r.split('=')[1].split('-')[0]) + 1
                                                  for r in source.requests))

    def test_incorrect_range_length_status_and_encoding_fail(self):
        for mode in ['wrong-range', 'short', 'overlong', 'whole-response', 'encoded']:
            with self.subTest(mode=mode):
                remote, _ = self.remote(archive(FIXTURE.read_bytes()), mode)
                with self.assertRaises((ValueError, zipfile.BadZipFile)):
                    product.acquire(remote, 'explicit-extra/opaque-original.raw', 1024)

    def test_missing_ambiguous_unsafe_and_oversized_members_fail(self):
        body = archive(FIXTURE.read_bytes())
        for name, limit in [('absent.original', 1024), ('../escape', 1024),
                            ('/absolute', 1024), ('explicit-extra/opaque-original.raw', 0)]:
            with self.subTest(name=name, limit=limit):
                remote, _ = self.remote(body)
                with self.assertRaises(ValueError):
                    product.acquire(remote, name, limit)
        remote, _ = self.remote(archive(b'original', duplicate=True))
        with self.assertRaisesRegex(ValueError, 'ambiguous'):
            product.acquire(remote, 'explicit-extra/opaque-original.raw', 1024)

    def test_corrupt_crc_and_local_header_member_mapping_fail(self):
        raw = FIXTURE.read_bytes()
        # ZIP_STORED makes the exact corruption location independent of deflate.
        target = io.BytesIO()
        with zipfile.ZipFile(target, 'w', zipfile.ZIP_STORED) as output:
            output.writestr('declared.raw', raw)
        body = target.getvalue()
        changed = bytearray(body)
        changed[body.index(raw)] ^= 1
        remote, _ = self.remote(bytes(changed))
        with self.assertRaises(zipfile.BadZipFile):
            product.acquire(remote, 'declared.raw', len(raw))
        changed = body.replace(b'declared.raw', b'misbound.raw', 1)
        remote, _ = self.remote(changed)
        with self.assertRaises(zipfile.BadZipFile):
            product.acquire(remote, 'declared.raw', len(raw))

    def test_metadata_and_member_reads_share_one_cumulative_deadline(self):
        clock = [0.0]
        deadline = product.Deadline(60, lambda: clock[0])
        # Represents time already spent by the same artifact's metadata owner.
        clock[0] = 58.0
        def tick():
            clock[0] += 1.0
        remote, source = self.remote(archive(FIXTURE.read_bytes()), deadline=deadline, tick=tick)
        with self.assertRaises(TimeoutError):
            product.acquire(remote, 'explicit-extra/opaque-original.raw', 1024)
        self.assertGreaterEqual(len(source.requests), 2)
        with self.assertRaises(TimeoutError):
            remote.read(1)


if __name__ == '__main__':
    unittest.main()
