import copy,json,sys
def upgrade(workflow):
    if workflow['schema_version'] != 1:
        raise ValueError('source workflow must be v1')
    out = copy.deepcopy(workflow)
    out['schema_version'] = 3
    out['historical_profiles'] = []
    return out
def convert(req):
    assert req['schema'] == 'chrono-historical-decode/v3'
    bindings = req['config_bindings']
    for endpoint, key in [('base','original'),('candidate','candidate')]:
        identity = bindings[endpoint]
        assert identity['entry_path'] == req['config_path']
        entry = req[key][identity['entry_path']]
        selection = identity['selection']
        assert selection['path'] == identity['entry_path']
        assert entry['platforms'][selection['platform']] == identity['effective_path'] == selection['config_path']
        assert req[key][identity['effective_path']]['schema_version'] == 3
    for path, value in req['original'].items():
        assert json.loads(bytes(req['original_bytes'][path])) == value
    a = req['original'][bindings['base']['effective_path']]
    b = req['candidate'][bindings['candidate']['effective_path']]
    assert a == b
    old = req['original'][a['registries']['workflow']]
    new = req['candidate'][b['registries']['workflow']]
    expected = upgrade(old)
    for key in ['historical_profiles','migrations','integration']:
        expected[key] = new[key]
    assert expected == new
    return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])
if __name__ == '__main__':
    req = json.load(sys.stdin)
    with open('.chrono-harness/state/decoder-input.json','w') as f:
        json.dump(req,f)
    print(json.dumps(convert(req)))
