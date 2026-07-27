import argparse
import gzip
import hashlib
import json
from pathlib import Path


SARAWAK_CITIES = ("bintulu", "kuching", "miri", "sibu")


def release_asset_name(manifest_path: str) -> str:
    return f"{manifest_path.replace('/', '--')}.gz"


def prepare(repository: Path, output: Path) -> list[Path]:
    maps: list[tuple[str, bytes]] = []
    for city in SARAWAK_CITIES:
        manifest_path = f"data/system/my/{city}/maps/center.bin"
        source = repository / Path(manifest_path)
        if not source.is_file():
            raise FileNotFoundError(f"Missing generated map for {city}: {source}")
        maps.append((manifest_path, source.read_bytes()))

    manifest_path = repository / "data" / "MANIFEST.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    output.mkdir(parents=True, exist_ok=True)
    assets = []
    for path, payload in maps:
        compressed = gzip.compress(payload, compresslevel=9, mtime=0)
        asset = output / release_asset_name(path)
        asset.write_bytes(compressed)
        assets.append(asset)
        manifest["entries"][path] = {
            "checksum": hashlib.md5(payload).hexdigest(),
            "uncompressed_size_bytes": len(payload),
            "compressed_size_bytes": len(compressed),
        }

    manifest["entries"] = dict(sorted(manifest["entries"].items()))
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return assets


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    for asset in prepare(repository, args.output.resolve()):
        print(asset)


if __name__ == "__main__":
    main()
