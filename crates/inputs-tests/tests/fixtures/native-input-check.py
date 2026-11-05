
import subprocess
assert subprocess.run([os.environ['FACTS_GIT'], '--version'], stdout=subprocess.PIPE).returncode == 0
assert os.environ.get('DECLARED_EMPTY') == '' and 'DECLARED_ABSENT' not in os.environ
print('data-present', os.path.exists(__EXTERNAL_INPUT__))
