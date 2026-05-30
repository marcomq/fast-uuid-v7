import unittest
import uuid

import fastuuidv7


class FastUuidV7Tests(unittest.TestCase):
    def test_gen_id_returns_int(self):
        self.assertIsInstance(fastuuidv7.gen_id(), int)

    def test_gen_id_str_is_uuid_v7(self):
        parsed = uuid.UUID(fastuuidv7.gen_id_str())
        self.assertEqual(parsed.version, 7)

    def test_gen_id_bytes_round_trips(self):
        raw = fastuuidv7.gen_id_bytes()
        self.assertIsInstance(raw, bytes)
        self.assertEqual(len(raw), 16)
        parsed = uuid.UUID(bytes=raw)
        self.assertEqual(parsed.version, 7)

    def test_format_uuid_round_trips(self):
        parsed = uuid.UUID(fastuuidv7.format_uuid(fastuuidv7.gen_id()))
        self.assertEqual(parsed.version, 7)

    def test_format_uuid_rejects_invalid_input(self):
        for value in ("not-an-int", -1, 1 << 128):
            with self.subTest(value=value):
                with self.assertRaises((TypeError, ValueError, OverflowError)) as ctx:
                    fastuuidv7.format_uuid(value)
                self.assertTrue(str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
