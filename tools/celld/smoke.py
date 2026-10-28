"""Exercise Rust/Wasm and persistent storage in an isolated celld project."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest
from urllib.error import URLError
from urllib.request import Request, urlopen


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


class LocalRuntimeTest(unittest.TestCase):
    def test_rust_wasm_and_durable_state_survive_container_recreation(self):
        subprocess.run(["sh", str(HERE / "build.sh")], check=True)
        with tempfile.TemporaryDirectory(prefix="celld-smoke-") as temporary:
            project = Path(temporary) / "project"
            shutil.copytree(
                HERE / "fixture", project,
                ignore=shutil.ignore_patterns(".celld", ".dev.vars*", ".env*"),
            )
            environment = dict(
                os.environ, CELLD_PROJECT_DIR=str(project), CELLD_PORT="0"
            )
            command = [
                "docker", "compose", "-f", str(ROOT / "compose.celld.yaml"),
                "-p", f"celld-smoke-{os.getpid()}",
            ]

            def compose(*arguments):
                return subprocess.run(
                    command + list(arguments), env=environment, check=True,
                    capture_output=True, text=True, timeout=120,
                ).stdout.strip()

            def wait_for_probe(url, expected):
                deadline = time.monotonic() + 30
                result = None
                while time.monotonic() < deadline:
                    try:
                        with urlopen(url, timeout=1) as response:
                            result = json.load(response)
                        if result == {"runtime": "celld", "wasm": expected}:
                            return
                    except (URLError, TimeoutError, ConnectionError):
                        pass
                    time.sleep(0.2)
                self.fail(f"celld probe did not reach {expected} within 30s: {result}")

            def start(expected):
                compose("up", "-d")
                address = compose("port", "celld", "9876")
                url = f"http://{address}"
                wait_for_probe(url, expected)
                return url

            def increment(url):
                request = Request(f"{url}/counter", method="POST")
                with urlopen(request, timeout=5) as response:
                    return json.load(response)["value"]

            try:
                url = start(42)
                self.assertEqual(increment(url), 1)
                self.assertEqual(increment(url), 2)
                source = project / "worker.mjs"
                source.write_text(
                    source.read_text().replace("probe.add(19, 23)", "probe.add(20, 23)")
                )
                wait_for_probe(url, 43)
                self.assertEqual(increment(url), 3)
                compose("down")
                url = start(43)
                self.assertEqual(increment(url), 4)
            except Exception:
                print(compose("logs", "--tail", "40"))
                raise
            finally:
                compose("down")


if __name__ == "__main__":
    unittest.main(verbosity=2)
