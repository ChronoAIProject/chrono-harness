import runpy
assert runpy.run_path('extra/product.py')['answer']()==42
assert open('.chrono-harness/state/order').read().count('p')==1
open('.chrono-harness/state/extra','w').write('passed')
