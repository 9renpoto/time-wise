"""Exercise the generated Rust Worker in an isolated, synthetic celld project."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

SERVER = Path(__file__).resolve().parents[1]
ROOT = SERVER.parents[1]


class WorkerSmokeTest(unittest.TestCase):
    def test_discovery_and_fail_closed_routes(self):
        subprocess.run(["sh", str(SERVER / "scripts/build.sh")], check=True)
        with tempfile.TemporaryDirectory(prefix="rust-worker-smoke-") as directory:
            project = Path(directory)
            shutil.copytree(SERVER / "build", project / "build")
            config = json.loads((SERVER / "wrangler.jsonc").read_text())
            config.pop("build")
            (project / "wrangler.jsonc").write_text(json.dumps(config))
            environment = dict(os.environ, CELLD_PROJECT_DIR=str(project), CELLD_PORT="0")
            command = ["docker", "compose", "-f", str(ROOT / "compose.celld.yaml"),
                       "-p", f"rust-worker-smoke-{os.getpid()}"]

            def compose(*args):
                return subprocess.run(command + list(args), env=environment, check=True,
                                      capture_output=True, text=True, timeout=120).stdout.strip()

            def request(url, path, method="GET"):
                try:
                    response = urlopen(Request(url + path, method=method), timeout=2)
                except HTTPError as error:
                    response = error
                with response:
                    return response.status, json.load(response), response.headers

            def start(expected):
                compose("up", "-d")
                url = "http://" + compose("port", "celld", "9876")
                deadline = time.monotonic() + 30
                while time.monotonic() < deadline:
                    try:
                        if request(url, "/v1/auth/config")[0] == expected:
                            return url
                    except (URLError, TimeoutError, ConnectionError):
                        pass
                    time.sleep(0.2)
                self.fail("Worker did not become ready within 30 seconds")

            try:
                url = start(200)
                status, body, headers = request(url, "/v1/auth/config")
                self.assertEqual(status, 200)
                self.assertEqual(body, {"protocol_version": 1,
                                       "auth_origin": "https://auth.example.com",
                                       "client_profile_ids": ["official-development-v1"]})
                self.assertEqual(headers["Cache-Control"], "no-store")
                self.assertIsNone(headers["Access-Control-Allow-Origin"])
                self.assertEqual(request(url, "/v1/auth/config", "POST")[0], 405)
                self.assertEqual(request(url, "/v1/auth/config", "POST")[2]["Allow"], "GET")
                for path, method in [("/v1/auth/attempts", "POST"),
                                     ("/auth/github/callback?code=synthetic&state=synthetic", "GET")]:
                    self.assertEqual(request(url, path, method)[:2],
                                     (503, {"error": "authentication_unavailable"}))
                self.assertEqual(request(url, "/unknown")[0], 404)
                compose("down")
                config["vars"]["AUTH_ORIGIN"] = "http://invalid.example.com"
                (project / "wrangler.jsonc").write_text(json.dumps(config))
                url = start(503)
                self.assertEqual(request(url, "/v1/auth/config")[:2],
                                 (503, {"error": "authentication_unavailable"}))
            except Exception:
                print(compose("logs", "--tail", "40"))
                raise
            finally:
                compose("down")


if __name__ == "__main__":
    unittest.main(verbosity=2)
