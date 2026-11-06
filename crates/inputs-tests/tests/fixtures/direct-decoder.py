import copy,json,sys
def upgrade(config):
    if config['schema_version'] != 1:
        raise ValueError('source config must be v1')
    out = copy.deepcopy(config)
    out['schema_version'] = 2
    for item in out['environment']['inputs']:
        if item['sha256'] is None:
            raise ValueError('unbound digest is not absence')
        item['presence'] = 'present'
    return out
def convert(req):
    assert req['schema'] == 'chrono-historical-decode/v2'
    for path, value in req['original'].items():
        assert json.loads(bytes(req['original_bytes'][path])) == value
    assert upgrade(req['original'][req['config_path']]) == req['candidate'][req['config_path']]
    return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])
if __name__ == '__main__':
    print(json.dumps(convert(json.load(sys.stdin))))
