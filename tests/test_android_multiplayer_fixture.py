import unittest
import re
from android_multiplayer_fixture import activity_state, divergent_cells


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


# Condensed from real `dumpsys activity activities com.sudokurivals` output (API 36 emulator).
RESUMED = '    topResumedActivity=ActivityRecord{107267860 u0 com.sudokurivals/.MainActivity t87}\n    * Hist  #0: ActivityRecord{107267860 u0 com.sudokurivals/.MainActivity t87}\n      packageName=com.sudokurivals processName=com.sudokurivals\n      launchedFromUid=0 launchedFromPackage=com.android.shell launchedFromFeature=null userId=0\n      ...\n      state=RESUMED delayedResume=false finishing=false\n      * Hist  #1: ActivityRecord{250433246 u0 com.google.android.apps.nexuslauncher/.NexusLauncherActivity t5}\n        packageName=com.google.android.apps.nexuslauncher processName=com.google.android.apps.nexuslauncher\n        launchedFromUid=0 launchedFromPackage=null launchedFromFeature=null userId=0\n      ...\n        state=STOPPED delayedResume=false finishing=false'
STOPPED = '      topResumedActivity=ActivityRecord{250433246 u0 com.google.android.apps.nexuslauncher/.NexusLauncherActivity t5}\n      * Hist  #1: ActivityRecord{250433246 u0 com.google.android.apps.nexuslauncher/.NexusLauncherActivity t5}\n        packageName=com.google.android.apps.nexuslauncher processName=com.google.android.apps.nexuslauncher\n        launchedFromUid=0 launchedFromPackage=null launchedFromFeature=null userId=0\n      ...\n        state=RESUMED delayedResume=false finishing=false\n    * Hist  #0: ActivityRecord{107267860 u0 com.sudokurivals/.MainActivity t87}\n      packageName=com.sudokurivals processName=com.sudokurivals\n      launchedFromUid=0 launchedFromPackage=com.android.shell launchedFromFeature=null userId=0\n      ...\n      state=STOPPED delayedResume=false finishing=false'


class ActivityStateTests(unittest.TestCase):
    def test_launcher_stopped_record_is_not_the_apps_state(self):
        # The old `state=STOPPED` search was true here although the app is still RESUMED,
        # so background() returned before onStop/resume could run.
        self.assertTrue(re.search(r'\bstate=STOPPED\b', RESUMED))
        self.assertEqual(activity_state(RESUMED, 'com.sudokurivals'), 'RESUMED')

    def test_stopped_app_with_resumed_launcher(self):
        self.assertTrue(re.search(r'\bstate=RESUMED\b', STOPPED))
        self.assertEqual(activity_state(STOPPED, 'com.sudokurivals'), 'STOPPED')

    def test_staging_package_record_is_not_the_debug_activity(self):
        staging = RESUMED.replace('com.sudokurivals/.MainActivity', 'com.sudokurivals.staging/com.sudokurivals.MainActivity')
        self.assertIsNone(activity_state(staging, 'com.sudokurivals'))

    def test_missing_record(self):
        self.assertIsNone(activity_state('', 'com.sudokurivals'))


if __name__ == '__main__':
    unittest.main()
