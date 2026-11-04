"""Python checks for the explicit workflow transition interface fixture."""
import runpy,json,copy
f=runpy.run_path('.chrono-harness/decoder.py')['convert']
r=json.load(open('.chrono-harness/case.json'))
o=f(r)
assert o['values']['.chrono-harness/FILEMAP.json']['schema_version']==2
assert o['values']['.chrono-harness/FILEMAP.json']['execution_plans']['test:t']['operations']==['prepare.p','execute.t']
bad=copy.deepcopy(r)
bad['profile_value']['schema']='unsupported'
try:
 f(bad)
except ValueError:
 pass
else:
 raise AssertionError('bad profile accepted')
