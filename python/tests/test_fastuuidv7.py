import threading
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

    def test_uuid_variant_is_derived_from_clock_sequence_bits(self):
        cases = (
            (0x00, "reserved for NCS compatibility"),
            (0x80, "specified in RFC 4122"),
            (0xC0, "reserved for Microsoft compatibility"),
            (0xE0, "reserved for future definition"),
        )

        for clock_seq_hi, expected in cases:
            with self.subTest(clock_seq_hi=clock_seq_hi):
                raw = clock_seq_hi << 56
                self.assertEqual(fastuuidv7.UUID(raw).variant, expected)

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


class SequentialGeneratorTests(unittest.TestCase):
    def test_ids_are_strictly_increasing(self):
        gen = fastuuidv7.SequentialGenerator()
        ids = [gen.next_id() for _ in range(300_000)]
        self.assertTrue(all(ids[i] < ids[i + 1] for i in range(len(ids) - 1)))

    def test_ids_are_uuid_v7(self):
        gen = fastuuidv7.SequentialGenerator()
        for _ in range(100):
            parsed = uuid.UUID(int=gen.next_id())
            self.assertEqual(parsed.version, 7)
            self.assertEqual(parsed.variant, uuid.RFC_4122)

    def test_strings_sort_in_generation_order(self):
        gen = fastuuidv7.SequentialGenerator()
        ids = [gen.next_id_str() for _ in range(10_000)]
        self.assertEqual(ids, sorted(ids))

    def test_bytes_sort_in_generation_order(self):
        gen = fastuuidv7.SequentialGenerator()
        ids = [gen.next_id_bytes() for _ in range(10_000)]
        self.assertEqual(len(ids[0]), 16)
        self.assertEqual(ids, sorted(ids))

    def test_next_uuid_returns_uuid_object(self):
        gen = fastuuidv7.SequentialGenerator()
        first = gen.next_uuid()
        second = gen.next_uuid()
        self.assertIsInstance(first, fastuuidv7.UUID)
        self.assertLess(first, second)

    def test_accessors_share_one_sequence(self):
        gen = fastuuidv7.SequentialGenerator()
        values = [
            gen.next_id(),
            int(uuid.UUID(gen.next_id_str())),
            int.from_bytes(gen.next_id_bytes(), "big"),
            gen.next_uuid().int,
        ]
        self.assertEqual(values, sorted(values))
        self.assertEqual(len(set(values)), len(values))

    def test_instances_are_independent(self):
        first, second = (
            fastuuidv7.SequentialGenerator(),
            fastuuidv7.SequentialGenerator(),
        )
        self.assertNotEqual(first.next_id(), second.next_id())

    def test_ordering_survives_thread_handoff(self):
        gen = fastuuidv7.SequentialGenerator()
        ids = [gen.next_id()]

        def take_one():
            ids.append(gen.next_id())

        for _ in range(8):
            thread = threading.Thread(target=take_one)
            thread.start()
            thread.join()

        self.assertEqual(ids, sorted(ids))
        self.assertEqual(len(set(ids)), len(ids))


if __name__ == "__main__":
    unittest.main()
