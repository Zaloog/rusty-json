import json
from time import perf_counter
from pathlib import Path

from rusty_json import sum_as_string, load_json

JSON = """
{
"key": "value",
"hi": [1, 3, "blub"],
"nix": null
}
"""

def load_twitter_json():
    file_path = Path('input.txt')
    if not file_path.exists():
        print("load data from url")
        import urllib.request
        names_url = 'https://raw.githubusercontent.com/serde-rs/json-benchmark/refs/heads/master/data/twitter.json'
        urllib.request.urlretrieve(names_url, 'input.txt')

    return file_path.read_text()




def read_python(json_str: str):
    print("Converting with pyhon...")
    start = perf_counter()
    result = json.loads(json_str)
    end = perf_counter()
    print(f"python took {end - start}s")
    return result

def read_rust(json_str: str):
    print("Converting with rust...")
    start = perf_counter()
    result = load_json(json_str)
    end = perf_counter()
    print(f"rust took {end - start}s")
    return result

if __name__ == "__main__":
    json_content = load_twitter_json()
    rust_json = read_rust(json_content)
    python_json = read_python(json_content)


    print(f"{sum_as_string(1, 2) = }")
