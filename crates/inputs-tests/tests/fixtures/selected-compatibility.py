import copy,json,runpy
module = runpy.run_path('.chrono-harness/decoder.py')
upgrade,convert = module['upgrade'],module['convert']
source = dict(schema_version=1,migrations=[])
assert upgrade(source) == dict(schema_version=3,migrations=[],historical_profiles=[])
assert source == dict(schema_version=1,migrations=[])
try:
    upgrade(dict(schema_version=3))
except ValueError:
    pass
else:
    raise AssertionError('accepted wrong historical schema')
# Exercise compatibility on the actual retained decoder input as well.
req = json.load(open('.chrono-harness/state/decoder-input.json'))
assert convert(req)['values'] == req['original']
for endpoint in ['base','candidate']:
    bad = copy.deepcopy(req)
    del bad['config_bindings'][endpoint]
    try:
        convert(bad)
    except KeyError:
        pass
    else:
        raise AssertionError('missing endpoint binding accepted')
print('selected workflow migration compatibility checked')
