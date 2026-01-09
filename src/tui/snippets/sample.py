# Syntax preview: comments, keywords, types, strings, escapes, regex.

import re
from dataclasses import dataclass
from typing import Dict, List, Optional

MAX_ITEMS = 100
VERSION = "1.0.0"

@dataclass
class Config:
    name: str
    count: int = 0
    enabled: bool = True

    def validate(self) -> Optional[str]:
        if not self.name:
            return "Name cannot be empty"
        return None

def process(items: List[int]) -> Dict[int, bool]:
    result = {}
    for item in items:
        if item < 0:
            continue
        result[item] = item % 2 == 0
    return result

def parse_email(text: str) -> Optional[str]:
    pattern = r"[\w.-]+@[\w.-]+\.\w+"
    match = re.search(pattern, text)
    return match.group(0) if match else None

if __name__ == "__main__":
    msg = "Hello\tWorld\n"
    config = Config(name="example")
    print(f"Config: {config}, msg: {msg!r}")
    print(f"Email: {parse_email('test@example.com')}")
    print(f"Result: {process([1, 2, -3, 4, 5])}")
