import runpy,sys,os,json
f=runpy.run_path('p/product.py')['double']
assert f(21)==42
assert open('.chrono-harness/state/order').read().endswith('p')
print(json.dumps([sys.argv[1:],os.getcwd(),os.environ.get('EMPTY'),os.environ.get('ABSENT')]))
open('.chrono-harness/state/order','a').write('t')
