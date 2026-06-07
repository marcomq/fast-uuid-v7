import unittest
import uuid

import fastuuidv7


class FastUuidV7Tests(unittest.TestCase):
    def test_gen_id_returns_int(self):
        self.assertIsInstance(fastuuidv7.gen_id(), int)

    def test_gen_id_with_sub_ms_variants_return_uuid_v7_ints(self):
        for name in (
            "gen_id_with_sub_ms_4",
            "gen_id_with_sub_ms_8",
            "gen_id_with_sub_ms_12",
        ):
            with self.subTest(name=name):
                raw = getattr(fastuuidv7, name)()
                self.assertIsInstance(raw, int)
                parsed = uuid.UUID(int=raw)
                self.assertEqual(parsed.version, 7)

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

    def test_uuid7_returns_uuid_compatible_object(self):
        generated = fastuuidv7.uuid7()

        self.assertIsInstance(generated, fastuuidv7.UUID)
        parsed = uuid.UUID(str(generated))
        self.assertEqual(parsed.version, 7)
        self.assertEqual(generated.int, parsed.int)
        self.assertEqual(int(generated), parsed.int)
        self.assertEqual(generated.bytes, parsed.bytes)
        self.assertEqual(generated.hex, parsed.hex)
        self.assertEqual(generated.urn, parsed.urn)
        self.assertEqual(generated.fields, parsed.fields)
        self.assertEqual(generated.version, 7)
        self.assertEqual(generated.timestamp, generated.time)

    def test_uuid_object_constructs_from_supported_values(self):
        raw = fastuuidv7.gen_id()
        text = fastuuidv7.format_uuid(raw)

        self.assertEqual(fastuuidv7.UUID(raw), fastuuidv7.UUID(text))
        self.assertEqual(fastuuidv7.format_uuid(fastuuidv7.UUID(raw)), text)

    def test_uuid7_does_not_mutate_held_uuid(self):
        first = fastuuidv7.uuid7()
        first_int = first.int
        second = fastuuidv7.uuid7()

        self.assertIsNot(first, second)
        self.assertEqual(first.int, first_int)
        self.assertNotEqual(first.int, second.int)

    def test_uuid7_str_preserves_string_alias(self):
        self.assertIsInstance(fastuuidv7.uuid7_str(), str)

    def test_uuid7_hex_is_undashed_uuid_hex(self):
        raw = fastuuidv7.uuid7_hex()
        self.assertIsInstance(raw, str)
        self.assertEqual(len(raw), 32)
        parsed = uuid.UUID(hex=raw)
        self.assertEqual(parsed.version, 7)

    def test_format_uuid_rejects_invalid_input(self):
        for value in ("not-an-int", -1, 1 << 128):
            with self.subTest(value=value):
                with self.assertRaises((TypeError, ValueError, OverflowError)) as ctx:
                    fastuuidv7.format_uuid(value)
                self.assertTrue(str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
