"""Small independent example program."""

def greeting(text):
    return text.strip()

if __name__ == "__main__":
    from pathlib import Path
    print(greeting(Path("message.txt").read_text()))
