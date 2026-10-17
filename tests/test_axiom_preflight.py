import copy
import unittest

from scripts.axiom_production_preflight import check


def valid_config():
    return {
        "hosts": {
            "app": "https://app.axiom.example.org",
            "api": "https://api.axiom.example.org",
            "events": "wss://events.axiom.example.org",
            "autumn": "https://files.axiom.example.org",
            "january": "https://proxy.axiom.example.org",
        },
        "api": {
            "smtp": {
                "host": "smtp.mail.example.org",
                "password": "unique-secret-not-for-real-use",
                "use_starttls": True,
                "from_address": "noreply@axiom.example.org",
            },
            "livekit": {"nodes": {"worldwide": {"secret": "a-different-random-value"}}},
        },
        "files": {"s3": {
            "secret_access_key": "another-unique-test-value",
            "access_key_id": "axiom_test",
        }},
    }


class ProductionPreflightTests(unittest.TestCase):
    def test_representative_secure_config(self):
        self.assertEqual([], check(valid_config()))

    def test_rejects_dev_urls(self):
        config = valid_config()
        config["hosts"]["events"] = "ws://local.revolt.chat:14703"
        self.assertTrue(any("hosts.events" in e for e in check(config)))

    def test_rejects_fixed_dev_storage_credentials(self):
        config = valid_config()
        config["files"]["s3"]["secret_access_key"] = "minioautumn"
        self.assertTrue(any("files.s3.secret_access_key" in e for e in check(config)))

    def test_rejects_mock_smtp(self):
        config = valid_config()
        config["api"]["smtp"].update(host="localhost", use_starttls=False)
        self.assertTrue(any("api.smtp" in e for e in check(config)))

    def test_rejects_missing_fields(self):
        config = valid_config()
        del config["hosts"]["api"]
        self.assertTrue(any("hosts.api" in e for e in check(config)))


if __name__ == "__main__":
    unittest.main()
