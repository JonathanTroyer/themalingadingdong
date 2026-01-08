"""Example Python code for syntax highlighting preview."""

import json
import re
from dataclasses import dataclass
from typing import Optional, Dict, List

MAX_RETRIES = 3
API_URL = "https://api.example.com"

@dataclass
class Config:
    name: str
    enabled: bool = True
    retries: int = MAX_RETRIES

    def validate(self) -> Optional[str]:
        if not self.name:
            return "Name cannot be empty"
        if self.retries > 10:
            return f"Retries {self.retries} exceeds maximum"
        return None

def process_items(items: List[int]) -> Dict[int, bool]:
    result = {}
    for item in items:
        is_even = item % 2 == 0
        result[item] = is_even
    return result

def parse_email(text: str) -> Optional[str]:
    pattern = r"[\w\.-]+@[\w\.-]+\.\w+"
    match = re.search(pattern, text)
    return match.group(0) if match else None

def main():
    config = Config(name="example")
    error = config.validate()
    if error:
        print(f"Error: {error}")
        return

    items = [1, 2, 3, 4, 5]
    processed = process_items(items)
    print(f"Processed: {json.dumps(processed)}")

if __name__ == "__main__":
    main()
