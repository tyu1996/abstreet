import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "prepare_sarawak_data.py"
SPEC = importlib.util.spec_from_file_location("prepare_sarawak_data", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PrepareSarawakDataTests(unittest.TestCase):
    def make_repository(self, root: Path, missing: str | None = None) -> dict[str, bytes]:
        payloads = {}
        for city in MODULE.SARAWAK_CITIES:
            if city == missing:
                continue
            payload = f"map for {city}".encode()
            path = root / "data" / "system" / "my" / city / "maps" / "center.bin"
            path.parent.mkdir(parents=True)
            path.write_bytes(payload)
            payloads[city] = payload

        manifest = {
            "entries": {
                "data/system/us/seattle/maps/montlake.bin": {
                    "checksum": "existing",
                    "uncompressed_size_bytes": 1,
                    "compressed_size_bytes": 1,
                }
            }
        }
        manifest_path = root / "data" / "MANIFEST.json"
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        return payloads

    def test_release_asset_name_encodes_the_manifest_path(self):
        self.assertEqual(
            MODULE.release_asset_name("data/system/my/kuching/maps/center.bin"),
            "data--system--my--kuching--maps--center.bin.gz",
        )

    def test_prepare_compresses_all_maps_and_updates_only_their_manifest_entries(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            payloads = self.make_repository(root)
            output = root / "dist"

            MODULE.prepare(root, output)

            manifest = json.loads((root / "data" / "MANIFEST.json").read_text())
            self.assertIn("data/system/us/seattle/maps/montlake.bin", manifest["entries"])
            for city, payload in payloads.items():
                path = f"data/system/my/{city}/maps/center.bin"
                entry = manifest["entries"][path]
                self.assertEqual(entry["checksum"], hashlib.md5(payload).hexdigest())
                self.assertEqual(entry["uncompressed_size_bytes"], len(payload))
                asset = output / MODULE.release_asset_name(path)
                self.assertEqual(gzip.decompress(asset.read_bytes()), payload)
                self.assertEqual(entry["compressed_size_bytes"], asset.stat().st_size)

    def test_prepare_rejects_an_incomplete_city_set_before_writing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.make_repository(root, missing="miri")
            original = (root / "data" / "MANIFEST.json").read_bytes()

            with self.assertRaisesRegex(FileNotFoundError, "miri"):
                MODULE.prepare(root, root / "dist")

            self.assertEqual((root / "data" / "MANIFEST.json").read_bytes(), original)
            self.assertFalse((root / "dist").exists())

    def test_checked_in_boundaries_are_compact_closed_polygons(self):
        repository = Path(__file__).parents[2]
        for city in MODULE.SARAWAK_CITIES:
            boundary = json.loads(
                (repository / "importer" / "config" / "my" / city / "center.geojson")
                .read_text(encoding="utf-8")
            )
            points = boundary["features"][0]["geometry"]["coordinates"][0]
            self.assertEqual(points[0], points[-1])
            longitudes = [point[0] for point in points]
            latitudes = [point[1] for point in points]
            self.assertLessEqual(max(longitudes) - min(longitudes), 0.10)
            self.assertLessEqual(max(latitudes) - min(latitudes), 0.08)

    def test_checked_in_osm_snapshots_are_valid_gzip_xml(self):
        repository = Path(__file__).parents[2]
        for city in MODULE.SARAWAK_CITIES:
            source = repository / "importer" / "config" / "my" / city / "center.osm.gz"
            with gzip.open(source, "rb") as compressed:
                osm = compressed.read()
            self.assertIn(b"<osm", osm[:200])
            self.assertIn(b"</osm>", osm[-200:])


if __name__ == "__main__":
    unittest.main()
