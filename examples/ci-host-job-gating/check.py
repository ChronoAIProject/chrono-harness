from pathlib import Path
from message import greeting

assert greeting(Path("message.txt").read_text()) == "hello"
