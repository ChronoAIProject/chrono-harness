import runpy
upgrade = runpy.run_path('.chrono-harness/decoder.py')['upgrade']
source = dict(schema_version=1,environment=dict(inputs=[dict(id='empty',location='x',sha256='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855')]))
expected = dict(schema_version=2,environment=dict(inputs=[dict(id='empty',location='x',sha256='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',presence='present')]))
assert upgrade(source) == expected
assert source['schema_version'] == 1 and 'presence' not in source['environment']['inputs'][0]
source['environment']['inputs'][0]['sha256'] = None
try:
    upgrade(source)
except ValueError:
    pass
else:
    raise AssertionError('unbound digest became absence')
print('config migration compatibility checked')
