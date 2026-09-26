from pathlib import Path

assert Path("message.txt").read_text() == "hello\n", "message must contain hello"
print("registered example check executed")
