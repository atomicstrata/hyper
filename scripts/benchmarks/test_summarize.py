import json
from pathlib import Path
import tempfile
import unittest
from summarize import summarize

class Summaries(unittest.TestCase):
    def test_completed_browser_rows_and_average_throughput(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'browser').mkdir()
            (root / 'browser/results.json').write_text(json.dumps({'runs': [{'dataset':'tiny','tool':'sigma','mode':'camera-motion','status':'completed','trace':{'rafIntervalMs':[10,10,40]}}]}))
            row = summarize(root)[0]
            self.assertEqual(row['successful_runs'], 1)
            self.assertEqual(row['frame_p50_ms'], 10)
            self.assertEqual(row['average_fps'], 50)

    def test_native_layout_excludes_post_update_steps_and_failed_runs(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'native').mkdir()
            common = {'dataset':'tiny','hulls':True,'live':True,'warmup':False}
            measured = {**common,'status':'ok','measurements':{
                'frames':300,'frame_interval_ms':[20]*300,
                'layout_step_ms':list(range(1,301))+[10000]*4,
                'update_commit_ms':[5], 'update_next_frame_ms':25}}
            failed = {**common,'status':'watchdog_timeout'}
            (root / 'native/raw.json').write_text(json.dumps([measured,failed]))
            row = summarize(root)[0]
            self.assertEqual(row['attempts'],2)
            self.assertEqual(row['successful_runs'],1)
            self.assertEqual(row['layout_step_p50_ms'],150.5)
            self.assertEqual(row['average_fps'],50)

if __name__ == '__main__': unittest.main()
