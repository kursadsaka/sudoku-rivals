import unittest
from android_multiplayer_fixture import divergent_cells


class DivergentFixtureTests(unittest.TestCase):
    def test_remote_differentiates_uncertain_ack_from_conflict(self):
        for size in (6, 9):
            for pending_value in (None, 1, 2):
                with self.subTest(size=size, pending=pending_value):
                    snapshot = {'size': size, 'own': {'givens': [size, 0], 'cells': [size, 0]}}
                    draft = {'cells': [{'value': size}, {'value': 2}]}
                    pending = None if pending_value is None else {
                        'envelope': {'command': {'type': 'sync', 'cells': [size, pending_value]}}}
                    remote, index = divergent_cells(snapshot, draft, pending)
                    self.assertEqual(index, 1)
                    self.assertEqual(remote[0], size)
                    self.assertNotEqual(remote, snapshot['own']['cells'])
                    self.assertNotEqual(remote, [size, 2])
                    if pending is not None:
                        self.assertNotEqual(remote, pending['envelope']['command']['cells'])
                    self.assertEqual(snapshot['own']['cells'], [size, 0])


if __name__ == '__main__':
    unittest.main()
